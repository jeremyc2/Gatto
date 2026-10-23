//! Process-lifetime update state, shared by the window and menu bar.
use gpui_kit::{App, AppContext as _, Context, Entity, Global};

use crate::{diagnostics::Diagnostics, menu_bar::MenuBarController, settings::ReleaseChannel};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UpdateState {
    #[default]
    Idle,
    Available,
    Installing,
    Failed,
}

#[derive(Default)]
pub struct Updater {
    pub state: UpdateState,
    pub message: String,
    channel: ReleaseChannel,
    generation: isize,
    menu: MenuBarController,
    #[cfg(target_os = "macos")]
    native: Option<native::Bridge>,
}

struct SharedUpdater(Entity<Updater>);
impl Global for SharedUpdater {}

impl Updater {
    pub fn shared(cx: &mut App) -> Entity<Self> {
        if let Some(shared) = cx.try_global::<SharedUpdater>() {
            return shared.0.clone();
        }
        let entity = cx.new(|_| Self::default());
        cx.set_global(SharedUpdater(entity.clone()));
        entity
    }

    pub fn start(
        cx: &mut App,
        diagnostics: Diagnostics,
        menu: MenuBarController,
        channel: ReleaseChannel,
    ) {
        let entity = Self::shared(cx);
        entity.update(cx, |this, _| {
            this.channel = channel;
            this.menu = menu.clone();
        });
        #[cfg(target_os = "macos")]
        {
            let (sender, events) = async_channel::unbounded();
            match native::Bridge::load(sender, channel) {
                Ok(Some(bridge)) => entity.update(cx, |this, _| this.native = Some(bridge)),
                Ok(None) => return, // Source builds have no bundled updater.
                Err(error) => {
                    diagnostics.error(format!("Could not start updater: {error:#}"));
                    return;
                }
            }
            cx.spawn(async move |cx| {
                while let Ok((generation, code, message)) = events.recv().await {
                    cx.update(|cx| {
                        if generation != entity.read(cx).generation {
                            return;
                        }
                        if code == 6 {
                            // Confirmation uses the existing upload guards and install path.
                            cx.dispatch_action(&crate::application_menu::InstallUpdate);
                            return;
                        }
                        if matches!(code, 3..=5) {
                            diagnostics.error(&message);
                        }
                        entity.update(cx, |this, cx| {
                            this.receive(code, message, cx);
                            menu.set_update_state(this.state);
                        });
                    });
                }
            })
            .detach();
        }
        #[cfg(not(target_os = "macos"))]
        let _ = (entity, diagnostics, menu);
    }

    fn receive(&mut self, code: isize, message: String, cx: &mut Context<Self>) {
        if code == 5 {
            if self.state == UpdateState::Idle {
                self.message = message;
                cx.notify();
            }
            return;
        }
        self.state = match code {
            0 | 4 => UpdateState::Idle,
            1 => UpdateState::Available,
            2 => UpdateState::Installing,
            3 => UpdateState::Failed,
            _ => return, // Background errors belong in logs, not in the badge.
        };
        self.message = message;
        cx.notify();
    }

    pub fn install(&mut self, cx: &mut Context<Self>) {
        if !matches!(self.state, UpdateState::Available | UpdateState::Failed) {
            return;
        }
        #[cfg(target_os = "macos")]
        if let Some(bridge) = &self.native {
            // Block new uploads in this turn, before the native event channel runs.
            self.state = UpdateState::Installing;
            self.message = "Downloading update; Gatto will restart when ready".into();
            cx.notify();
            bridge.install();
        }
        #[cfg(not(target_os = "macos"))]
        let _ = cx;
    }

    pub fn check_for_updates(&self) -> bool {
        #[cfg(target_os = "macos")]
        if let Some(bridge) = &self.native {
            bridge.check_for_updates();
            return true;
        }
        false
    }

    pub fn generation(&self) -> isize {
        self.generation
    }

    pub fn set_channel(
        &mut self,
        channel: ReleaseChannel,
        cx: &mut Context<Self>,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.state != UpdateState::Installing,
            "Finish the update before changing release channels."
        );
        if self.channel == channel {
            return Ok(());
        }
        self.channel = channel;
        self.generation = self.generation.wrapping_add(1);
        self.state = UpdateState::Idle;
        self.message = "Checking the selected release channel…".into();
        self.menu.invalidate_update_offer();
        #[cfg(target_os = "macos")]
        if let Some(bridge) = &self.native {
            bridge.set_channel(channel, self.generation);
        } else {
            self.message =
                "Channel saved. Update checks are available in packaged releases.".into();
        }
        cx.notify();
        Ok(())
    }
}

#[cfg(target_os = "macos")]
mod native {
    use anyhow::{Context as _, Result};
    use block2::RcBlock;
    use objc2::{MainThreadMarker, extern_class, extern_methods, rc::Retained, runtime::AnyClass};
    use objc2_foundation::{NSObject, NSString};

    extern_class!(
        /// Implemented by the bundled Swift class in native/Updater.swift.
        /// Its @MainActor methods are available only through a main-thread owner.
        #[unsafe(super(NSObject))]
        #[thread_kind = objc2::MainThreadOnly]
        #[name = "GattoUpdaterBridge"]
        struct NativeUpdater;
    );

    // SAFETY: The selectors and argument/return types match Updater.swift. Swift
    // copies the escaping block and strings; NSString callback arguments are
    // nonnull and borrowed only during that invocation. MainThreadOnly enforces
    // the Swift class's actor requirement on all Rust callers.
    impl NativeUpdater {
        extern_methods!(
            #[unsafe(method(new))]
            fn new(mtm: MainThreadMarker) -> Retained<Self>;

            #[unsafe(method(configureChannel:generation:))]
            fn configure(&self, channel: &NSString, generation: isize);

            #[unsafe(method(startWithHandler:))]
            fn start(
                &self,
                handler: &block2::Block<dyn Fn(isize, isize, std::ptr::NonNull<NSString>)>,
            );

            #[unsafe(method(install))]
            fn install(&self);

            #[unsafe(method(checkForUpdates))]
            fn check_for_updates(&self);
        );
    }

    unsafe extern "C" {
        fn dlopen(path: *const std::ffi::c_char, mode: std::ffi::c_int) -> *mut std::ffi::c_void;
        fn dlerror() -> *const std::ffi::c_char;
    }

    pub struct Bridge {
        object: Retained<NativeUpdater>,
    }

    impl Bridge {
        pub fn load(
            events: async_channel::Sender<(isize, isize, String)>,
            channel: crate::settings::ReleaseChannel,
        ) -> Result<Option<Self>> {
            let executable = std::env::current_exe()?;
            let Some(contents) = executable.parent().and_then(|path| path.parent()) else {
                return Ok(None);
            };
            if contents.file_name().is_none_or(|name| name != "Contents") {
                return Ok(None);
            }
            let library = contents.join("Frameworks/libGattoUpdater.dylib");
            let main_thread = MainThreadMarker::new()
                .context("The updater must be started on the application main thread")?;
            // SAFETY: Only load the bridge inside our signed app bundle. Hardened
            // runtime library validation checks its signing team at runtime.
            use std::os::unix::ffi::OsStrExt as _;
            let path = std::ffi::CString::new(library.as_os_str().as_bytes())?;
            // Darwin RTLD_NOW (0x2) | RTLD_LOCAL (0x4) resolves the signed bridge
            // before using its class without exporting its C/Swift symbols.
            // Intentionally never dlclose: registered Objective-C classes and
            // callbacks must remain loaded for the lifetime of this process.
            if unsafe { dlopen(path.as_ptr(), 0x2 | 0x4) }.is_null() {
                let error = unsafe { dlerror() };
                let detail = if error.is_null() {
                    "unknown dynamic loader error".into()
                } else {
                    unsafe { std::ffi::CStr::from_ptr(error) }.to_string_lossy()
                };
                anyhow::bail!("Could not load the signed updater bridge: {detail}");
            }
            AnyClass::get(c"GattoUpdaterBridge").context("Missing updater bridge class")?;
            let object = NativeUpdater::new(main_thread);
            let handler = RcBlock::new(
                move |generation: isize, code: isize, message: std::ptr::NonNull<NSString>| {
                    // SAFETY: The typed Swift callback supplies a nonnull string
                    // borrowed for this invocation; copy it before returning.
                    let message = unsafe { message.as_ref() }.to_string();
                    let _ = events.try_send((generation, code, message));
                },
            );
            object.configure(&NSString::from_str(channel.key()), 0);
            object.start(&handler);
            Ok(Some(Self { object }))
        }

        pub fn set_channel(&self, channel: crate::settings::ReleaseChannel, generation: isize) {
            self.object
                .configure(&NSString::from_str(channel.key()), generation);
        }

        pub fn install(&self) {
            self.object.install();
        }

        pub fn check_for_updates(&self) {
            self.object.check_for_updates();
        }
    }
}
