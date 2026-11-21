import "./rooftops-controls.css";
import * as stylex from "@stylexjs/stylex";
import { styles } from "./rooftops.stylex";
import { drawMedalToCanvas, MEDAL_HEIGHT, MEDAL_WIDTH } from "./achievement-artwork";
import {
  achievementDescription,
  achievementLabel,
  ALL_ACHIEVEMENTS,
  countAchievements,
  hasAchievement,
} from "./achievements";
import { downloadScorecardPng, renderPosterToCanvas } from "./art";
import {
  AudioEngine,
  CUE_COIN,
  CUE_RETURN,
  CUE_SHUTTERS,
  FINALE,
  MENU,
  type AudioScene,
} from "./audio/engine";
import {
  computeCourseViewport,
  computeTouchControlsLayout,
  CourseRenderer,
  drawCourseHud,
  type CourseViewport,
  type TouchControlState,
} from "./course-renderer";
import { Flight, harborFrame, HarborRenderer, menuTitleLayout } from "./harbor/flight";
import { aimFromPointer, defaultAim, followAim, type Aim } from "./harbor/projection";
import { arrivalFlamesOn, ghostGap, LevelSlide, Run } from "./simulation";
import {
  frameAt,
  loadGameData,
  Recording,
  saveAchievements,
  saveCompletion,
  savePreferences,
} from "./storage";
import {
  ARRIVAL_DURATION,
  ARRIVAL_LANDING,
  clamp,
  LEVELS,
  START_X,
  STEP,
  type Preferences,
  type Replay,
  type Score,
} from "./types";

if (import.meta.env.DEV) {
  void import("virtual:stylex:css-only");
}

function setStyleX(
  element: HTMLElement,
  props: ReturnType<typeof stylex.attrs>,
  extraClass?: string,
): void {
  const cls = props.class ?? "";
  element.className = extraClass ? (cls ? `${extraClass} ${cls}` : extraClass) : cls;
}

function setVisible(element: HTMLElement, visible: boolean): void {
  element.hidden = !visible;
}

interface LockableOrientation {
  lock?: (orientation: "landscape" | "portrait") => Promise<void>;
  unlock?: () => void;
}

function canProgrammaticallyLockLandscape(): boolean {
  const orientation = (typeof screen !== "undefined" ? screen.orientation : undefined) as
    LockableOrientation | undefined;
  return typeof orientation?.lock === "function";
}

async function tryLockLandscape(): Promise<boolean> {
  const orientation = (typeof screen !== "undefined" ? screen.orientation : undefined) as
    LockableOrientation | undefined;
  if (typeof orientation?.lock !== "function") return false;
  try {
    await orientation.lock("landscape");
    return true;
  } catch {
    // Android Chromium/Firefox require fullscreen before locking orientation.
    if (
      document.fullscreenElement === null &&
      typeof document.documentElement.requestFullscreen === "function"
    ) {
      try {
        await document.documentElement.requestFullscreen();
        await orientation.lock("landscape");
        return true;
      } catch {
        return false;
      }
    }
    return false;
  }
}

function createBackArrowIcon(): SVGSVGElement {
  const svgNS = "http://www.w3.org/2000/svg";
  const svg = document.createElementNS(svgNS, "svg");
  svg.setAttribute("width", "15");
  svg.setAttribute("height", "15");
  svg.setAttribute("viewBox", "0 0 16 16");
  svg.setAttribute("fill", "none");
  svg.setAttribute("stroke", "currentColor");
  svg.setAttribute("stroke-width", "2");
  svg.setAttribute("stroke-linecap", "round");
  svg.setAttribute("stroke-linejoin", "round");
  svg.setAttribute("aria-hidden", "true");
  svg.style.display = "block";
  svg.style.flexShrink = "0";

  const stem = document.createElementNS(svgNS, "path");
  stem.setAttribute("d", "M13 8H3.5");
  const head = document.createElementNS(svgNS, "path");
  head.setAttribute("d", "M7.5 4L3.5 8L7.5 12");
  svg.append(stem, head);
  return svg;
}

function createRotatePhoneIcon(size: number): SVGSVGElement {
  const svgNS = "http://www.w3.org/2000/svg";
  const svg = document.createElementNS(svgNS, "svg");
  svg.setAttribute("width", String(size));
  svg.setAttribute("height", String(size));
  svg.setAttribute("viewBox", "0 0 24 24");
  svg.setAttribute("fill", "none");
  svg.setAttribute("stroke", "currentColor");
  svg.setAttribute("stroke-width", "1.9");
  svg.setAttribute("stroke-linecap", "round");
  svg.setAttribute("stroke-linejoin", "round");
  svg.setAttribute("aria-hidden", "true");
  svg.style.display = "block";
  svg.style.flexShrink = "0";

  const phone = document.createElementNS(svgNS, "rect");
  phone.setAttribute("x", "3");
  phone.setAttribute("y", "8");
  phone.setAttribute("width", "14");
  phone.setAttribute("height", "9");
  phone.setAttribute("rx", "2");
  const arc = document.createElementNS(svgNS, "path");
  arc.setAttribute("d", "M20 11a7 7 0 0 0-7-7");
  const arrow = document.createElementNS(svgNS, "path");
  arrow.setAttribute("d", "M17.5 8.5L20 11l2-2.5");
  svg.append(phone, arc, arrow);
  return svg;
}

function createGearIcon(): SVGSVGElement {
  const svgNS = "http://www.w3.org/2000/svg";
  const svg = document.createElementNS(svgNS, "svg");
  svg.setAttribute("width", "21");
  svg.setAttribute("height", "21");
  svg.setAttribute("viewBox", "0 0 24 24");
  svg.setAttribute("fill", "none");
  svg.setAttribute("stroke", "currentColor");
  svg.setAttribute("stroke-width", "2");
  svg.setAttribute("stroke-linecap", "round");
  svg.setAttribute("stroke-linejoin", "round");
  svg.setAttribute("aria-hidden", "true");

  const circle = document.createElementNS(svgNS, "circle");
  circle.setAttribute("cx", "12");
  circle.setAttribute("cy", "12");
  circle.setAttribute("r", "3");
  svg.appendChild(circle);

  const path = document.createElementNS(svgNS, "path");
  path.setAttribute(
    "d",
    "M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09a1.65 1.65 0 0 0-1-1.51 1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09a1.65 1.65 0 0 0 1.51-1 1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33h.08a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82v.08a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z",
  );
  svg.appendChild(path);
  return svg;
}

class RooftopsGame {
  private readonly root: HTMLElement;
  private readonly canvas: HTMLCanvasElement;
  private readonly ctx: CanvasRenderingContext2D;
  private readonly courseRenderer = new CourseRenderer();
  private readonly harborRenderer = new HarborRenderer();
  private readonly audio = new AudioEngine();

  private run: Run = Run.forArrival();
  private menuOpen = true;
  private menuTime = 0;
  private menuCamera: Aim = defaultAim();
  private menuAim: Aim = defaultAim();
  private menuFlight: Flight | null = null;
  private arrival: number | null = 0;
  private slide: LevelSlide | null = null;
  private started = false;
  private hudChangedMs: number | null = null;
  private optionsOpen = false;
  private achievementsOpen = false;
  private rotatePromptOpen = false;
  private viewingScorecard: Score | null = null;

  private preferences: Preferences;
  private preferencesMessage: string | null = null;
  private personalBest: number | null;
  private runPersonalBest: number | null;
  private bestScorecard: Score | null;
  private bestReplay: Replay | null;
  private ghostReplay: Replay | null;
  private achievements: number;
  private recording: Recording;
  private completionRecorded = false;

  private musicPreview = false;
  private effectsInitialVolume: number | null = null;
  private effectsAdjustmentKind: "pointer" | "keyboard" | null = null;

  private savingScore = false;
  private saveMessage: string | null = null;

  private viewportW = 900;
  private viewportH = 560;
  private dpr = 1;
  private safeTop = 0;
  private safeRight = 0;
  private safeBottom = 0;
  private safeLeft = 0;
  private lastFrameMs: number | null = null;
  private remainder = 0;
  private camera = 0;
  private windowFocused = true;
  private systemReducedMotion = false;
  private touchMode = false;
  private movePointerId: number | null = null;
  private joyOriginX = 0;
  private joyOriginY = 0;
  private joyKnobX = 0;
  private joyKnobY = 0;
  private moveDirection: -1 | 0 | 1 = 0;
  private jumpPointerId: number | null = null;
  private rafId: number | null = null;
  private needsRender = true;
  private lastScreenKey = "";

  private backLink!: HTMLAnchorElement;
  private menuActionsWrap!: HTMLDivElement;
  private menuScorecardBtn!: HTMLButtonElement;
  private optionsBtn!: HTMLButtonElement;

  private resultsControlsEl!: HTMLDivElement;
  private downloadScoreBtn!: HTMLButtonElement;
  private resultsMessageEl!: HTMLDivElement;

  private rotateDialog!: HTMLDialogElement;
  private rotateBodyEl!: HTMLParagraphElement;
  private rotateEnterBtn!: HTMLButtonElement;

  private optionsDialog!: HTMLDialogElement;
  private musicSlider!: HTMLInputElement;
  private musicValueEl!: HTMLSpanElement;
  private effectsSlider!: HTMLInputElement;
  private effectsValueEl!: HTMLSpanElement;
  private ghostCheckbox!: HTMLInputElement;
  private animationsCheckbox!: HTMLInputElement;
  private optionsAudioErrEl!: HTMLParagraphElement;
  private optionsSaveErrEl!: HTMLParagraphElement;
  private optionsMainMenuBtn!: HTMLButtonElement;
  private optionsContinueBtn!: HTMLButtonElement;

  private achievementsDialog!: HTMLDialogElement;
  private achievementsCountEl!: HTMLParagraphElement;
  private achievementsGridEl!: HTMLDivElement;

  constructor(root: HTMLElement) {
    this.root = root;
    this.root.tabIndex = -1;
    setStyleX(this.root, stylex.attrs(styles.root));

    const stored = loadGameData();
    this.preferences = stored.preferences;
    this.personalBest = stored.personalBest;
    this.runPersonalBest = stored.personalBest;
    this.bestScorecard = stored.scorecard;
    this.bestReplay = stored.replay;
    this.ghostReplay = stored.replay;
    this.achievements = stored.achievements;
    this.recording = new Recording(this.run.ghostFrame());

    const motionQuery = window.matchMedia("(prefers-reduced-motion: reduce)");
    this.systemReducedMotion = motionQuery.matches;
    motionQuery.addEventListener("change", (event) => {
      this.systemReducedMotion = event.matches;
      this.requestFrame();
    });

    const coarseQuery = window.matchMedia("(pointer: coarse)");
    this.touchMode = coarseQuery.matches;
    coarseQuery.addEventListener("change", (event) => {
      this.touchMode = event.matches;
      this.syncLandscapeGate();
      this.requestFrame();
    });

    this.windowFocused = !document.hidden;

    this.canvas = document.createElement("canvas");
    setStyleX(this.canvas, stylex.attrs(styles.canvas));
    this.canvas.setAttribute("role", "img");
    this.canvas.setAttribute("aria-label", "Gatto Rooftops game canvas");
    const ctx = this.canvas.getContext("2d", { alpha: false });
    if (!ctx) throw new Error("Canvas 2D context is unavailable");
    this.ctx = ctx;
    this.root.appendChild(this.canvas);

    this.buildUi();
    this.bindEvents();
    this.audio.setStatusListener(() => this.syncOptionsUi());
    this.syncViewport();
    this.syncScreenChrome(true);
    this.requestFrame();
  }

  private buildUi(): void {
    this.backLink = document.createElement("a");
    setStyleX(this.backLink, stylex.attrs(styles.backLink));
    this.backLink.href = "/";
    const backLabel = document.createElement("span");
    backLabel.textContent = "Gatto";
    this.backLink.append(createBackArrowIcon(), backLabel);
    this.root.appendChild(this.backLink);

    this.menuActionsWrap = document.createElement("div");
    setStyleX(this.menuActionsWrap, stylex.attrs(styles.menuActionsWrap));
    this.menuActionsWrap.setAttribute("role", "navigation");
    this.menuActionsWrap.setAttribute("aria-label", "Rooftops main menu");

    const menuActions = document.createElement("div");
    setStyleX(menuActions, stylex.attrs(styles.menuActions));

    const playBtn = document.createElement("button");
    playBtn.type = "button";
    setStyleX(playBtn, stylex.attrs(styles.btn, styles.btnPrimary, styles.menuBtnSized));
    playBtn.textContent = "Play";
    playBtn.addEventListener("click", () => {
      this.audio.unlock();
      this.playFromMenu();
    });

    const settingsBtn = document.createElement("button");
    settingsBtn.type = "button";
    setStyleX(settingsBtn, stylex.attrs(styles.btn, styles.btnMenu, styles.menuBtnSized));
    settingsBtn.textContent = "Settings";
    settingsBtn.addEventListener("click", () => {
      this.audio.unlock();
      this.openOptions();
    });

    const achievementsBtn = document.createElement("button");
    achievementsBtn.type = "button";
    setStyleX(achievementsBtn, stylex.attrs(styles.btn, styles.btnMenu, styles.menuBtnSized));
    achievementsBtn.textContent = "Achievements";
    achievementsBtn.addEventListener("click", () => {
      this.audio.unlock();
      this.openAchievements();
    });

    this.menuScorecardBtn = document.createElement("button");
    this.menuScorecardBtn.type = "button";
    setStyleX(this.menuScorecardBtn, stylex.attrs(styles.btn, styles.btnMenu, styles.menuBtnSized));
    this.menuScorecardBtn.textContent = "Scorecard";
    this.menuScorecardBtn.addEventListener("click", () => {
      this.audio.unlock();
      if (this.bestScorecard) this.showScorecard(this.bestScorecard);
    });

    menuActions.append(playBtn, settingsBtn, achievementsBtn, this.menuScorecardBtn);
    this.menuActionsWrap.appendChild(menuActions);
    this.root.appendChild(this.menuActionsWrap);

    this.optionsBtn = document.createElement("button");
    this.optionsBtn.type = "button";
    setStyleX(this.optionsBtn, stylex.attrs(styles.optionsBtn));
    this.optionsBtn.setAttribute("aria-label", "Options");
    this.optionsBtn.title = "Options";
    this.optionsBtn.appendChild(createGearIcon());
    this.optionsBtn.addEventListener("click", () => {
      this.audio.unlock();
      this.openOptions();
    });
    this.root.appendChild(this.optionsBtn);

    this.resultsControlsEl = document.createElement("div");
    setStyleX(this.resultsControlsEl, stylex.attrs(styles.resultsControls));

    const resultsActions = document.createElement("div");
    setStyleX(resultsActions, stylex.attrs(styles.resultsActions));

    const resultsMainMenuBtn = document.createElement("button");
    resultsMainMenuBtn.type = "button";
    setStyleX(resultsMainMenuBtn, stylex.attrs(styles.btn, styles.btnPrimary));
    resultsMainMenuBtn.textContent = "Main menu";
    resultsMainMenuBtn.addEventListener("click", () => {
      this.audio.unlock();
      this.returnToMenu();
    });

    this.downloadScoreBtn = document.createElement("button");
    this.downloadScoreBtn.type = "button";
    setStyleX(this.downloadScoreBtn, stylex.attrs(styles.btn, styles.btnPrimary));
    this.downloadScoreBtn.textContent = "Download scorecard";
    this.downloadScoreBtn.addEventListener("click", () => {
      void this.handleDownloadScorecard();
    });

    resultsActions.append(resultsMainMenuBtn, this.downloadScoreBtn);
    this.resultsMessageEl = document.createElement("div");
    setStyleX(this.resultsMessageEl, stylex.attrs(styles.resultsMessage));
    this.resultsControlsEl.append(resultsActions, this.resultsMessageEl);
    this.root.appendChild(this.resultsControlsEl);

    this.optionsDialog = document.createElement("dialog");
    setStyleX(this.optionsDialog, stylex.attrs(styles.dialog), "rooftops-dialog");
    this.optionsDialog.setAttribute("closedby", "any");
    this.optionsDialog.setAttribute("aria-labelledby", "rooftops-options-title");

    const optionsCard = document.createElement("div");
    setStyleX(optionsCard, stylex.attrs(styles.optionsCard));

    const optTitle = document.createElement("h2");
    optTitle.id = "rooftops-options-title";
    setStyleX(optTitle, stylex.attrs(styles.optionsTitle));
    optTitle.textContent = "Rooftops";

    const musicRow = this.buildVolumeRow("rooftops-music", "Music", "music");
    this.musicSlider = musicRow.slider;
    this.musicValueEl = musicRow.valueEl;

    const effectsRow = this.buildVolumeRow("rooftops-effects", "Sound effects", "effects");
    this.effectsSlider = effectsRow.slider;
    this.effectsValueEl = effectsRow.valueEl;

    const ghostRow = this.buildCheckboxRow("rooftops-ghost", "Ghost cat", (checked) => {
      this.preferences.ghost = checked;
      this.persistPreferences();
      this.requestFrame();
    });
    this.ghostCheckbox = ghostRow.checkbox;

    const animRow = this.buildCheckboxRow("rooftops-animations", "Animations", (checked) => {
      this.preferences.animations = checked;
      if (!checked) {
        this.finishArrival();
        this.finishSlide();
      }
      this.persistPreferences();
      this.requestFrame();
    });
    this.animationsCheckbox = animRow.checkbox;

    this.optionsAudioErrEl = document.createElement("p");
    setStyleX(this.optionsAudioErrEl, stylex.attrs(styles.optionsError));
    this.optionsAudioErrEl.hidden = true;

    this.optionsSaveErrEl = document.createElement("p");
    setStyleX(this.optionsSaveErrEl, stylex.attrs(styles.optionsError));
    this.optionsSaveErrEl.hidden = true;

    const optActions = document.createElement("div");
    setStyleX(optActions, stylex.attrs(styles.optionsActions));

    this.optionsMainMenuBtn = document.createElement("button");
    this.optionsMainMenuBtn.type = "button";
    setStyleX(this.optionsMainMenuBtn, stylex.attrs(styles.btn, styles.btnMenu, styles.btnFlex));
    this.optionsMainMenuBtn.textContent = "Main menu";
    this.optionsMainMenuBtn.addEventListener("click", () => {
      this.closeOptions();
      this.returnToMenu();
    });

    this.optionsContinueBtn = document.createElement("button");
    this.optionsContinueBtn.type = "button";
    setStyleX(this.optionsContinueBtn, stylex.attrs(styles.btn, styles.btnPrimary, styles.btnFlex));
    this.optionsContinueBtn.textContent = "Done";
    this.optionsContinueBtn.addEventListener("click", () => this.closeOptions());

    optActions.append(this.optionsMainMenuBtn, this.optionsContinueBtn);
    optionsCard.append(
      optTitle,
      musicRow.row,
      effectsRow.row,
      ghostRow.row,
      animRow.row,
      this.optionsAudioErrEl,
      this.optionsSaveErrEl,
      optActions,
    );
    this.optionsDialog.appendChild(optionsCard);
    this.root.appendChild(this.optionsDialog);

    this.achievementsDialog = document.createElement("dialog");
    setStyleX(this.achievementsDialog, stylex.attrs(styles.dialog), "rooftops-dialog");
    this.achievementsDialog.setAttribute("closedby", "any");
    this.achievementsDialog.dataset.backdrop = "achievements";
    this.achievementsDialog.setAttribute("aria-labelledby", "rooftops-achievements-title");

    const achCard = document.createElement("div");
    setStyleX(achCard, stylex.attrs(styles.achievementsCard));

    const achHeader = document.createElement("div");
    setStyleX(achHeader, stylex.attrs(styles.achievementsHeader));

    const achTitleWrap = document.createElement("div");
    setStyleX(achTitleWrap, stylex.attrs(styles.achievementsTitleWrap));

    const achTitle = document.createElement("h2");
    achTitle.id = "rooftops-achievements-title";
    setStyleX(achTitle, stylex.attrs(styles.achievementsTitle));
    achTitle.textContent = "Achievements";

    this.achievementsCountEl = document.createElement("p");
    setStyleX(this.achievementsCountEl, stylex.attrs(styles.achievementsCount));
    achTitleWrap.append(achTitle, this.achievementsCountEl);

    const achCloseBtn = document.createElement("button");
    achCloseBtn.type = "button";
    setStyleX(achCloseBtn, stylex.attrs(styles.btn, styles.btnMenu));
    achCloseBtn.textContent = "Close";
    achCloseBtn.addEventListener("click", () => this.closeAchievements());

    achHeader.append(achTitleWrap, achCloseBtn);
    this.achievementsGridEl = document.createElement("div");
    setStyleX(this.achievementsGridEl, stylex.attrs(styles.achievementsGrid));
    achCard.append(achHeader, this.achievementsGridEl);
    this.achievementsDialog.appendChild(achCard);
    this.root.appendChild(this.achievementsDialog);

    // Full-screen opaque landscape gate whenever a mobile/touch device is in portrait.
    this.rotateDialog = document.createElement("dialog");
    setStyleX(this.rotateDialog, stylex.attrs(styles.rotateDialog), "rooftops-dialog");
    this.rotateDialog.setAttribute("closedby", "none");
    this.rotateDialog.dataset.backdrop = "opaque";
    this.rotateDialog.setAttribute("aria-labelledby", "rooftops-rotate-title");

    const rotateStage = document.createElement("div");
    setStyleX(rotateStage, stylex.attrs(styles.rotateStage));

    const rotateBackLink = document.createElement("a");
    setStyleX(rotateBackLink, stylex.attrs(styles.backLink));
    rotateBackLink.href = "/";
    const rotateBackLabel = document.createElement("span");
    rotateBackLabel.textContent = "Gatto";
    rotateBackLink.append(createBackArrowIcon(), rotateBackLabel);

    const rotateIcon = createRotatePhoneIcon(52);
    rotateIcon.style.color = "#ffdba0";

    const rotateTitle = document.createElement("h2");
    rotateTitle.id = "rooftops-rotate-title";
    setStyleX(rotateTitle, stylex.attrs(styles.rotateTitle));
    rotateTitle.textContent = "Rooftops";

    const canLock = canProgrammaticallyLockLandscape();
    this.rotateBodyEl = document.createElement("p");
    setStyleX(this.rotateBodyEl, stylex.attrs(styles.rotateBody));
    this.rotateBodyEl.textContent = "Turn your device sideways to play.";

    const rotateActions = document.createElement("div");
    setStyleX(rotateActions, stylex.attrs(styles.rotateActions));

    this.rotateEnterBtn = document.createElement("button");
    this.rotateEnterBtn.type = "button";
    setStyleX(this.rotateEnterBtn, stylex.attrs(styles.btn, styles.btnPrimary, styles.rotateBtn));
    this.rotateEnterBtn.textContent = "Rotate screen";
    this.rotateEnterBtn.hidden = !canLock;
    this.rotateEnterBtn.addEventListener("click", () => {
      this.audio.unlock();
      void tryLockLandscape().then((locked) => {
        if (!locked && this.needsLandscapeGate()) {
          this.rotateEnterBtn.hidden = true;
        }
        this.syncViewport();
      });
    });

    rotateActions.appendChild(this.rotateEnterBtn);
    rotateStage.append(rotateBackLink, rotateIcon, rotateTitle, this.rotateBodyEl, rotateActions);
    this.rotateDialog.appendChild(rotateStage);
    this.root.appendChild(this.rotateDialog);
  }

  private buildVolumeRow(
    id: string,
    label: string,
    kind: "music" | "effects",
  ): { row: HTMLDivElement; slider: HTMLInputElement; valueEl: HTMLSpanElement } {
    const row = document.createElement("div");
    setStyleX(row, stylex.attrs(styles.volumeRow));

    const header = document.createElement("div");
    setStyleX(header, stylex.attrs(styles.volumeHeader));
    const labelEl = document.createElement("label");
    labelEl.htmlFor = id;
    labelEl.textContent = label;
    const valueEl = document.createElement("span");
    setStyleX(valueEl, stylex.attrs(styles.volumeValue));
    header.append(labelEl, valueEl);

    const slider = document.createElement("input");
    slider.id = id;
    slider.type = "range";
    slider.className = "rooftops-slider";
    slider.min = "0";
    slider.max = "100";
    slider.step = "1";

    const syncSliderCss = () => {
      slider.style.setProperty("--attr-min", slider.min || "0");
      slider.style.setProperty("--attr-max", slider.max || "100");
      slider.style.setProperty("--control-value", slider.value);
      const val = Number(slider.value);
      const text = val === 0 ? "Off" : `${val}%`;
      valueEl.textContent = text;
      slider.setAttribute("aria-valuetext", text);
    };

    slider.addEventListener("pointerdown", () => {
      this.audio.unlock();
      if (!this.optionsOpen) return;
      this.audio.suspend();
      if (kind === "music") {
        this.musicPreview = true;
        this.syncMusicPreview(this.windowFocused);
      } else {
        this.effectsAdjustmentKind = "pointer";
        this.effectsInitialVolume = this.preferences.effectsVolume;
      }
    });

    const endPointer = () => {
      if (kind === "music") {
        this.endMusicPreview();
      } else {
        this.finishEffectsAdjustment("pointer");
      }
    };
    slider.addEventListener("pointerup", endPointer);
    slider.addEventListener("pointercancel", endPointer);

    slider.addEventListener("keydown", (event) => {
      const isSliderKey =
        event.key === "ArrowLeft" ||
        event.key === "ArrowRight" ||
        event.key === "ArrowUp" ||
        event.key === "ArrowDown" ||
        event.key === "Home" ||
        event.key === "End";
      if (!isSliderKey) return;
      this.audio.unlock();
      if (kind === "effects" && this.effectsAdjustmentKind !== "keyboard") {
        this.effectsAdjustmentKind = "keyboard";
        this.effectsInitialVolume = this.preferences.effectsVolume;
        this.musicPreview = false;
        this.audio.suspend();
      } else if (kind === "music") {
        this.musicPreview = true;
      }
      if (
        event.shiftKey &&
        (event.key === "ArrowLeft" ||
          event.key === "ArrowRight" ||
          event.key === "ArrowUp" ||
          event.key === "ArrowDown")
      ) {
        event.preventDefault();
        const current = Number(slider.value);
        const delta = event.key === "ArrowLeft" || event.key === "ArrowDown" ? -5 : 5;
        slider.value = String(clamp(current + delta, 0, 100));
        slider.dispatchEvent(new Event("input", { bubbles: true }));
      }
    });

    slider.addEventListener("keyup", (event) => {
      if (
        event.key === "ArrowLeft" ||
        event.key === "ArrowRight" ||
        event.key === "ArrowUp" ||
        event.key === "ArrowDown" ||
        event.key === "Home" ||
        event.key === "End"
      ) {
        if (kind === "music") {
          this.endMusicPreview();
        } else {
          this.finishEffectsAdjustment("keyboard");
        }
      }
    });

    slider.addEventListener("input", () => {
      this.audio.unlock();
      syncSliderCss();
      const val = Math.round(clamp(Number(slider.value), 0, 100));
      if (kind === "music") {
        this.preferences.musicVolume = val;
        this.audio.retry();
        if (this.optionsOpen && this.musicPreview) {
          this.syncMusicPreview(this.windowFocused);
        }
      } else {
        this.preferences.effectsVolume = val;
        this.audio.retry();
      }
      this.persistPreferences();
    });

    slider.addEventListener("blur", () => {
      if (kind === "music") this.endMusicPreview();
      else if (this.effectsAdjustmentKind !== null) {
        this.finishEffectsAdjustment(this.effectsAdjustmentKind);
      }
    });

    row.append(header, slider);
    return { row, slider, valueEl };
  }

  private buildCheckboxRow(
    id: string,
    label: string,
    onChange: (checked: boolean) => void,
  ): { row: HTMLLabelElement; checkbox: HTMLInputElement } {
    const row = document.createElement("label");
    setStyleX(row, stylex.attrs(styles.optionCheckboxRow));
    row.htmlFor = id;

    const checkbox = document.createElement("input");
    checkbox.id = id;
    checkbox.type = "checkbox";
    checkbox.className = "rooftops-checkbox";
    checkbox.addEventListener("change", () => {
      this.audio.unlock();
      onChange(checkbox.checked);
    });

    const text = document.createElement("span");
    text.textContent = label;
    row.append(checkbox, text);
    return { row, checkbox };
  }

  private bindEvents(): void {
    const supportsDevicePixelBox =
      typeof ResizeObserverEntry !== "undefined" &&
      "devicePixelContentBoxSize" in ResizeObserverEntry.prototype;
    const observer = new ResizeObserver((entries) => {
      const entry = entries[0];
      if (!entry) return;
      const rect = entry.contentRect;
      this.viewportW = Math.max(1, rect.width);
      this.viewportH = Math.max(1, rect.height);
      const dpc = entry.devicePixelContentBoxSize?.[0];
      const pixelW = dpc ? dpc.inlineSize : Math.round(this.viewportW * window.devicePixelRatio);
      const pixelH = dpc ? dpc.blockSize : Math.round(this.viewportH * window.devicePixelRatio);
      this.dpr = pixelW / this.viewportW;
      this.canvas.width = Math.max(1, pixelW);
      this.canvas.height = Math.max(1, pixelH);
      this.syncLayout();
      this.needsRender = true;
      this.requestFrame();
    });
    observer.observe(
      this.canvas,
      supportsDevicePixelBox ? { box: "device-pixel-content-box" } : {},
    );

    this.root.addEventListener("pointerdown", (event) => {
      this.windowFocused = !document.hidden;
      if (event.pointerType === "touch" && !this.touchMode) {
        this.touchMode = true;
        this.syncLayout();
      }
      this.audio.unlock();
    });

    this.canvas.addEventListener("pointerdown", (event) => this.handleTouchDown(event));
    this.canvas.addEventListener("pointermove", (event) => this.handleTouchMove(event));
    this.canvas.addEventListener("pointerup", (event) => this.handleTouchUp(event));
    this.canvas.addEventListener("pointercancel", (event) => this.handleTouchUp(event));

    this.root.addEventListener("pointermove", (event) => {
      if (!this.menuOpen || this.optionsOpen || this.achievementsOpen || this.reducedMotion()) {
        return;
      }
      const frame = harborFrame(this.viewportW, this.viewportH);
      const next = aimFromPointer(
        event.clientX - frame.x,
        event.clientY - frame.y,
        frame.width,
        frame.height,
      );
      if (next.x !== this.menuAim.x || next.y !== this.menuAim.y) {
        this.menuAim = next;
        this.requestFrame();
      }
    });

    window.addEventListener("keydown", (event) => this.handleKeyDown(event));
    window.addEventListener("keyup", (event) => this.handleKeyUp(event));

    const wireDialogDismiss = (dialog: HTMLDialogElement, onClose: () => void) => {
      dialog.addEventListener("cancel", (event) => {
        event.preventDefault();
        onClose();
      });
      dialog.addEventListener("close", () => {
        onClose();
      });
      // Fallback for browsers without native <dialog closedby="any"> support (e.g. Safari).
      if (!("closedBy" in HTMLDialogElement.prototype)) {
        let pointerDownOnBackdrop = false;
        const isOutside = (clientX: number, clientY: number) => {
          const rect = dialog.getBoundingClientRect();
          return (
            clientY < rect.top ||
            clientY > rect.top + rect.height ||
            clientX < rect.left ||
            clientX > rect.left + rect.width
          );
        };
        dialog.addEventListener("pointerdown", (event) => {
          pointerDownOnBackdrop =
            event.target === dialog && isOutside(event.clientX, event.clientY);
        });
        dialog.addEventListener("click", (event) => {
          if (!pointerDownOnBackdrop) return;
          pointerDownOnBackdrop = false;
          if (event.target !== dialog) return;
          if (!isOutside(event.clientX, event.clientY)) return;
          onClose();
        });
      }
    };

    wireDialogDismiss(this.optionsDialog, () => {
      if (this.optionsOpen) this.closeOptions();
    });
    wireDialogDismiss(this.achievementsDialog, () => {
      if (this.achievementsOpen) this.closeAchievements();
    });
    // The landscape gate cannot be dismissed until the device is actually in landscape.
    this.rotateDialog.addEventListener("cancel", (event) => {
      if (this.needsLandscapeGate()) {
        event.preventDefault();
      }
    });
    this.rotateDialog.addEventListener("close", () => {
      if (this.needsLandscapeGate()) {
        this.rotatePromptOpen = false;
        this.syncLandscapeGate();
      } else {
        this.rotatePromptOpen = false;
      }
    });

    const handleOrientationChange = () => {
      this.syncViewport();
      this.requestFrame();
    };
    window.addEventListener("orientationchange", handleOrientationChange);
    screen.orientation?.addEventListener?.("change", handleOrientationChange);

    const handleDeactivate = () => {
      this.windowFocused = false;
      this.clearTouchPointers();
      this.run.input = { left: false, right: false, jump: false };
      this.run.buffer = 0;
      this.lastFrameMs = null;
      this.remainder = 0;
      this.musicPreview = false;
      this.effectsAdjustmentKind = null;
      this.effectsInitialVolume = null;
      this.audio.suspend();
      this.requestFrame();
    };
    const handleActivate = () => {
      this.windowFocused = !document.hidden;
      this.lastFrameMs = null;
      this.remainder = 0;
      this.requestFrame();
    };

    window.addEventListener("blur", handleDeactivate);
    window.addEventListener("focus", handleActivate);
    document.addEventListener("visibilitychange", () => {
      if (document.hidden) handleDeactivate();
      else handleActivate();
    });
    window.addEventListener("pagehide", () => {
      this.audio.dispose();
      const orientation = (typeof screen !== "undefined" ? screen.orientation : undefined) as
        LockableOrientation | undefined;
      try {
        orientation?.unlock?.();
      } catch {
        // Ignore unlock failures.
      }
    });
  }

  private readSafeAreaInsets(): void {
    const style = getComputedStyle(document.documentElement);
    const px = (prop: string) => {
      const value = Number.parseFloat(style.getPropertyValue(prop));
      return Number.isFinite(value) ? value : 0;
    };
    this.safeTop = px("--sat");
    this.safeRight = px("--sar");
    this.safeBottom = px("--sab");
    this.safeLeft = px("--sal");
  }

  private needsLandscapeGate(): boolean {
    return this.touchMode && this.viewportH > this.viewportW;
  }

  private syncLandscapeGate(): void {
    const shouldGate = this.needsLandscapeGate();
    if (shouldGate) {
      this.clearTouchPointers();
      this.run.input = { left: false, right: false, jump: false };
      this.run.buffer = 0;
      this.lastFrameMs = null;
      this.remainder = 0;
      this.audio.suspend();
      if (!this.rotatePromptOpen || !this.rotateDialog.open) {
        this.rotatePromptOpen = true;
        if (!this.rotateDialog.open) {
          this.rotateDialog.showModal();
        }
      }
    } else if (this.rotatePromptOpen || this.rotateDialog.open) {
      this.rotatePromptOpen = false;
      this.lastFrameMs = null;
      this.remainder = 0;
      if (this.rotateDialog.open) {
        this.rotateDialog.close();
      }
      if (!this.optionsOpen && !this.achievementsOpen) {
        this.root.focus({ preventScroll: true });
      }
    }
  }

  private posterMetrics(): { scale: number; x: number; y: number; width: number; height: number } {
    const padX = this.viewportW < 560 ? 24 : 48;
    const padY = this.viewportH < 480 ? 78 : 100;
    const scale = Math.max(
      0.1,
      Math.min((this.viewportW - padX) / 900, (this.viewportH - padY) / 540),
    );
    const width = 900 * scale;
    const height = 540 * scale;
    const totalH = height + 12 + 44 + 20;
    const x = (this.viewportW - width) * 0.5;
    const y = Math.max(14, (this.viewportH - totalH) * 0.5);
    return { scale, x, y, width, height };
  }

  private syncViewport(): void {
    const rect = this.root.getBoundingClientRect();
    this.viewportW = Math.max(1, rect.width || window.innerWidth);
    this.viewportH = Math.max(1, rect.height || window.innerHeight);
    this.dpr = window.devicePixelRatio || 1;
    this.canvas.width = Math.max(1, Math.round(this.viewportW * this.dpr));
    this.canvas.height = Math.max(1, Math.round(this.viewportH * this.dpr));
    this.syncLayout();
  }

  private syncLayout(): void {
    this.readSafeAreaInsets();
    const layout = menuTitleLayout(this.viewportW, this.viewportH, this.touchMode);
    this.menuActionsWrap.style.left = `${layout.frameX}px`;
    this.menuActionsWrap.style.top = `${layout.actionsTop}px`;
    this.menuActionsWrap.style.width = `${layout.frameW}px`;
    this.menuActionsWrap.style.setProperty("--menu-scale", String(layout.scale));

    const poster = this.posterMetrics();
    this.resultsControlsEl.style.top = `${Math.round(poster.y + poster.height + 12)}px`;
    this.resultsControlsEl.style.width = `${Math.round(860 * poster.scale)}px`;

    this.syncLandscapeGate();
    this.syncScreenChrome(false);
  }

  private reducedMotion(): boolean {
    return !this.preferences.animations || this.systemReducedMotion;
  }

  private displayedScore(): Score | null {
    if (this.viewingScorecard) return this.viewingScorecard;
    if (this.run.home && this.run.level + 1 === LEVELS) return this.currentScore();
    return null;
  }

  private currentScore(): Score {
    return {
      ...this.run.score(),
      personalBest: this.runPersonalBest,
    };
  }

  private audioScene(active: boolean): AudioScene {
    const harbor = this.menuOpen || this.menuFlight !== null;
    const results = this.displayedScore() !== null;
    return {
      active,
      level: harbor ? MENU : results ? FINALE : this.run.level,
      resting: results || this.run.home || this.slide !== null,
      thruster: !harbor && !results && this.arrival !== null && arrivalFlamesOn(this.arrival),
    };
  }

  private syncAudio(active: boolean): void {
    const inHarbor = this.menuOpen || this.menuFlight !== null;
    this.audio.sync(this.audioScene(active), {
      ...this.preferences,
      effectsVolume: inHarbor ? 0 : this.preferences.effectsVolume,
    });
  }

  private syncMusicPreview(active: boolean): void {
    this.audio.sync(
      { ...this.audioScene(active), thruster: false },
      { ...this.preferences, effectsVolume: 0 },
    );
  }

  private endMusicPreview(): void {
    if (this.musicPreview) {
      this.musicPreview = false;
      this.audio.suspend();
    }
  }

  private finishEffectsAdjustment(kind: "pointer" | "keyboard"): void {
    if (this.effectsAdjustmentKind !== kind) return;
    const initial = this.effectsInitialVolume;
    this.effectsAdjustmentKind = null;
    this.effectsInitialVolume = null;
    if (
      this.optionsOpen &&
      this.windowFocused &&
      initial !== null &&
      this.preferences.effectsVolume !== initial
    ) {
      this.audio.previewEffects(this.preferences.effectsVolume);
    }
    this.persistPreferences();
  }

  private persistPreferences(): void {
    const ok = savePreferences(this.preferences);
    this.preferencesMessage = ok ? null : "Couldn't save changes. Adjust an option to try again.";
    this.syncOptionsUi();
  }

  private openOptions(): void {
    this.musicPreview = false;
    this.effectsAdjustmentKind = null;
    this.effectsInitialVolume = null;
    this.optionsOpen = true;
    this.clearTouchPointers();
    this.run.input = { left: false, right: false, jump: false };
    this.run.buffer = 0;
    this.lastFrameMs = null;
    this.remainder = 0;
    this.audio.suspend();
    this.syncOptionsUi();
    if (!this.optionsDialog.open) {
      this.optionsDialog.showModal();
    }
    this.syncScreenChrome(true);
    this.requestFrame();
  }

  private closeOptions(): void {
    this.endMusicPreview();
    if (this.effectsAdjustmentKind !== null) {
      this.effectsAdjustmentKind = null;
      this.effectsInitialVolume = null;
    }
    this.optionsOpen = false;
    this.lastFrameMs = null;
    this.remainder = 0;
    this.audio.suspend();
    if (this.optionsDialog.open) {
      this.optionsDialog.close();
    }
    this.root.focus({ preventScroll: true });
    this.syncScreenChrome(true);
    this.requestFrame();
  }

  private syncOptionsUi(): void {
    this.musicSlider.value = String(this.preferences.musicVolume);
    this.musicSlider.style.setProperty("--attr-min", "0");
    this.musicSlider.style.setProperty("--attr-max", "100");
    this.musicSlider.style.setProperty("--control-value", this.musicSlider.value);
    const mText = this.preferences.musicVolume === 0 ? "Off" : `${this.preferences.musicVolume}%`;
    this.musicValueEl.textContent = mText;
    this.musicSlider.setAttribute("aria-valuetext", mText);

    this.effectsSlider.value = String(this.preferences.effectsVolume);
    this.effectsSlider.style.setProperty("--attr-min", "0");
    this.effectsSlider.style.setProperty("--attr-max", "100");
    this.effectsSlider.style.setProperty("--control-value", this.effectsSlider.value);
    const eText =
      this.preferences.effectsVolume === 0 ? "Off" : `${this.preferences.effectsVolume}%`;
    this.effectsValueEl.textContent = eText;
    this.effectsSlider.setAttribute("aria-valuetext", eText);

    this.ghostCheckbox.checked = this.preferences.ghost;
    this.animationsCheckbox.checked = this.preferences.animations;

    const audioErr = this.audio.error();
    setVisible(this.optionsAudioErrEl, audioErr !== null);
    this.optionsAudioErrEl.textContent = audioErr ?? "";

    setVisible(this.optionsSaveErrEl, this.preferencesMessage !== null);
    this.optionsSaveErrEl.textContent = this.preferencesMessage ?? "";

    const inCourse = !this.menuOpen && this.displayedScore() === null;
    setVisible(this.optionsMainMenuBtn, inCourse);
    this.optionsContinueBtn.textContent = this.menuOpen ? "Done" : "Continue";
  }

  private openAchievements(): void {
    this.achievementsOpen = true;
    this.lastFrameMs = null;
    this.renderAchievementsGrid();
    if (!this.achievementsDialog.open) {
      this.achievementsDialog.showModal();
    }
    this.requestFrame();
  }

  private closeAchievements(): void {
    this.achievementsOpen = false;
    this.lastFrameMs = null;
    if (this.achievementsDialog.open) {
      this.achievementsDialog.close();
    }
    this.root.focus({ preventScroll: true });
    this.requestFrame();
  }

  private renderAchievementsGrid(): void {
    const unlockedCount = countAchievements(this.achievements);
    this.achievementsCountEl.textContent = `${unlockedCount} of ${ALL_ACHIEVEMENTS.length} unlocked`;
    this.achievementsGridEl.replaceChildren();

    for (const award of ALL_ACHIEVEMENTS) {
      const unlocked = hasAchievement(this.achievements, award);
      const card = document.createElement("div");
      setStyleX(card, stylex.attrs(styles.awardCard, unlocked && styles.awardCardUnlocked));

      const medal = document.createElement("canvas");
      setStyleX(medal, stylex.attrs(styles.awardMedal));
      medal.width = MEDAL_WIDTH * 2;
      medal.height = MEDAL_HEIGHT * 2;
      medal.setAttribute("aria-hidden", "true");
      drawMedalToCanvas(medal, award, unlocked);

      const label = document.createElement("div");
      setStyleX(label, stylex.attrs(styles.awardLabel, unlocked && styles.awardLabelUnlocked));
      label.textContent = achievementLabel(award);

      const desc = document.createElement("div");
      setStyleX(desc, stylex.attrs(styles.awardDesc));
      desc.textContent = achievementDescription(award);

      const status = document.createElement("div");
      setStyleX(status, stylex.attrs(styles.awardStatus, unlocked && styles.awardStatusUnlocked));
      status.textContent = unlocked ? "Unlocked" : "Locked";

      card.append(medal, label, desc, status);
      this.achievementsGridEl.appendChild(card);
    }
  }

  private resetRun(): void {
    this.clearTouchPointers();
    this.run = Run.forArrival();
    this.arrival = 0;
    this.slide = null;
    this.started = false;
    this.hudChangedMs = null;
    this.menuOpen = false;
    this.menuFlight = null;
    this.achievementsOpen = false;
    this.viewingScorecard = null;
    this.runPersonalBest = this.personalBest;
    this.ghostReplay = this.bestReplay;
    this.recording = new Recording(this.run.ghostFrame());
    this.lastFrameMs = null;
    this.remainder = 0;
    this.camera = 0;
    this.completionRecorded = false;
    this.saveMessage = null;
  }

  private playFromMenu(): void {
    if (!this.menuOpen || this.optionsOpen || this.achievementsOpen || this.rotatePromptOpen)
      return;
    this.resetRun();
    if (this.reducedMotion()) {
      this.finishArrival();
    } else {
      this.menuFlight = new Flight();
    }
    this.root.focus({ preventScroll: true });
    this.syncScreenChrome(true);
    this.requestFrame();
  }

  private returnToMenu(): void {
    this.audio.stop();
    this.resetRun();
    this.menuOpen = true;
    this.menuTime = 0;
    this.menuCamera = defaultAim();
    this.menuAim = defaultAim();
    this.root.focus({ preventScroll: true });
    this.syncScreenChrome(true);
    this.requestFrame();
  }

  private showScorecard(score: Score): void {
    this.menuOpen = false;
    this.menuFlight = null;
    this.achievementsOpen = false;
    this.menuTime = 0;
    this.viewingScorecard = score;
    this.saveMessage = null;
    if (this.optionsDialog.open) this.optionsDialog.close();
    this.optionsOpen = false;
    this.audio.stop();
    this.root.focus({ preventScroll: true });
    this.syncScreenChrome(true);
    this.requestFrame();
  }

  private continueFromHome(): void {
    this.clearTouchPointers();
    this.finishArrival();
    if (this.run.level + 1 === LEVELS) {
      this.returnToMenu();
      return;
    }
    const previous = this.run.clone();
    const previousCamera = this.camera;
    this.started = false;
    this.hudChangedMs = null;
    this.lastFrameMs = null;
    this.remainder = 0;
    this.saveMessage = null;
    this.completionRecorded = false;
    this.camera = 0;
    this.run.jumpDown();
    const offset = START_X - previous.destination();
    this.slide = new LevelSlide(previous, previousCamera + offset);
    this.camera = previousCamera + offset;
    this.recording.capture(this.run.ghostFrame());
    this.syncScreenChrome(true);
    this.requestFrame();
  }

  private finishMenuFlight(): void {
    if (this.menuFlight !== null) {
      this.menuFlight = null;
      this.lastFrameMs = null;
    }
  }

  private finishArrival(): void {
    if (this.arrival !== null) {
      this.arrival = null;
      this.run.lit[0] = true;
      this.lastFrameMs = null;
      this.remainder = 0;
    }
  }

  private finishSlide(): void {
    if (this.slide !== null) {
      this.run.time += this.slide.time;
      this.slide = null;
      this.camera = 0;
      this.lastFrameMs = null;
      this.remainder = 0;
    }
  }

  private beginPlay(nowMs: number): void {
    if (this.run.home) return;
    this.finishMenuFlight();
    this.finishArrival();
    this.finishSlide();
    if (!this.started) {
      this.started = true;
      this.hudChangedMs = nowMs;
      this.lastFrameMs = nowMs;
      this.remainder = 0;
    }
  }

  private handleKeyDown(event: KeyboardEvent): void {
    if (event.metaKey || event.ctrlKey || event.altKey) return;
    this.windowFocused = !document.hidden;

    if (event.key === "Escape") {
      if (this.rotatePromptOpen) {
        event.preventDefault();
      } else if (this.achievementsOpen) {
        event.preventDefault();
        this.closeAchievements();
      } else if (this.optionsOpen) {
        event.preventDefault();
        this.closeOptions();
      }
      return;
    }

    if (this.optionsOpen || this.achievementsOpen || this.rotatePromptOpen) return;

    const isSpace = event.code === "Space" || event.key === " ";
    const isLeft = event.key === "ArrowLeft";
    const isRight = event.key === "ArrowRight";
    if (!isSpace && !isLeft && !isRight) return;

    this.touchMode = false;
    this.audio.unlock();

    if (this.menuOpen || this.displayedScore() !== null) {
      if (isSpace && !event.repeat) {
        if (document.activeElement instanceof HTMLButtonElement) return;
        event.preventDefault();
        if (this.menuOpen) this.playFromMenu();
        else this.returnToMenu();
      }
      return;
    }

    event.preventDefault();
    const nowMs = performance.now();
    if (isLeft) {
      this.beginPlay(nowMs);
      this.run.input.left = true;
    } else if (isRight) {
      this.beginPlay(nowMs);
      this.run.input.right = true;
    } else if (isSpace && !event.repeat) {
      if (this.run.home) {
        this.continueFromHome();
      } else {
        this.beginPlay(nowMs);
        this.run.jumpDown();
      }
    }
    this.syncScreenChrome(false);
    this.requestFrame();
  }

  private handleKeyUp(event: KeyboardEvent): void {
    if (this.optionsOpen || this.menuOpen || this.viewingScorecard !== null) return;
    if (event.key === "ArrowLeft") {
      this.run.input.left = false;
    } else if (event.key === "ArrowRight") {
      this.run.input.right = false;
    } else if (event.code === "Space" || event.key === " ") {
      this.run.jumpUp();
    }
  }

  private currentTouchLayout() {
    return computeTouchControlsLayout(
      this.viewportW,
      this.viewportH,
      this.safeBottom,
      this.safeLeft,
      this.safeRight,
    );
  }

  private clearTouchPointers(): void {
    this.movePointerId = null;
    this.moveDirection = 0;
    this.jumpPointerId = null;
  }

  private updateJoystickFromPoint(x: number, y: number): void {
    const layout = this.currentTouchLayout();
    const maxOffset = layout.joyRadius - layout.knobRadius * 0.35;
    const dx = x - this.joyOriginX;
    const dy = y - this.joyOriginY;
    const dist = Math.hypot(dx, dy);
    if (dist > maxOffset && dist > 0) {
      const ratio = maxOffset / dist;
      this.joyKnobX = this.joyOriginX + dx * ratio;
      this.joyKnobY = this.joyOriginY + dy * ratio;
    } else {
      this.joyKnobX = x;
      this.joyKnobY = y;
    }

    // Joystick moves freely in 360°, while horizontal deflection drives left/right walking.
    const deadzone = 12;
    const dir: -1 | 0 | 1 = dx > deadzone ? 1 : dx < -deadzone ? -1 : 0;
    this.moveDirection = dir;
    if (dir !== 0) {
      this.beginPlay(performance.now());
    }
    this.run.input.left = dir === -1;
    this.run.input.right = dir === 1;
  }

  private handleTouchDown(event: PointerEvent): void {
    if (event.pointerType !== "touch") return;
    const wasTouch = this.touchMode;
    this.touchMode = true;
    if (!wasTouch) this.syncLayout();
    this.windowFocused = !document.hidden;
    this.audio.unlock();

    if (
      this.menuOpen ||
      this.optionsOpen ||
      this.achievementsOpen ||
      this.rotatePromptOpen ||
      this.displayedScore() !== null
    ) {
      return;
    }

    event.preventDefault();
    const rect = this.canvas.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;

    if (this.run.home) {
      this.continueFromHome();
      return;
    }

    const layout = this.currentTouchLayout();
    const distToJump = Math.hypot(x - layout.jumpX, y - layout.jumpY);
    const inJumpSide = distToJump <= layout.jumpRadius * 1.85 || x >= this.viewportW * 0.5;

    if (!inJumpSide && this.movePointerId === null) {
      this.movePointerId = event.pointerId;
      // Keep the stick anchored at its visible resting spot if tapped inside/near it,
      // or gently recenter if the thumb lands further out on the left half.
      const distToBase = Math.hypot(x - layout.joyBaseX, y - layout.joyBaseY);
      if (distToBase <= layout.joyRadius * 1.25) {
        this.joyOriginX = layout.joyBaseX;
        this.joyOriginY = layout.joyBaseY;
      } else {
        this.joyOriginX = clamp(x, layout.joyRadius + 10, this.viewportW * 0.45);
        this.joyOriginY = clamp(y, layout.joyRadius + 60, this.viewportH - layout.joyRadius - 10);
      }
      this.updateJoystickFromPoint(x, y);
      try {
        this.canvas.setPointerCapture(event.pointerId);
      } catch {
        // Synthetic pointer events in tests may not have an active pointer capture target.
      }
      this.syncScreenChrome(false);
      this.requestFrame();
    } else if (inJumpSide && this.jumpPointerId === null) {
      this.jumpPointerId = event.pointerId;
      this.beginPlay(performance.now());
      this.run.jumpDown();
      try {
        this.canvas.setPointerCapture(event.pointerId);
      } catch {
        // Ignore if capture is unavailable.
      }
      this.syncScreenChrome(false);
      this.requestFrame();
    }
  }

  private handleTouchMove(event: PointerEvent): void {
    if (event.pointerId !== this.movePointerId) return;
    event.preventDefault();
    const rect = this.canvas.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;
    this.updateJoystickFromPoint(x, y);
    this.syncScreenChrome(false);
    this.requestFrame();
  }

  private handleTouchUp(event: PointerEvent): void {
    if (event.pointerId === this.movePointerId) {
      this.movePointerId = null;
      this.moveDirection = 0;
      this.run.input.left = false;
      this.run.input.right = false;
      this.requestFrame();
    } else if (event.pointerId === this.jumpPointerId) {
      this.jumpPointerId = null;
      this.run.jumpUp();
      this.requestFrame();
    }
  }

  private async handleDownloadScorecard(): Promise<void> {
    const score = this.displayedScore();
    if (!score || this.savingScore) return;
    this.savingScore = true;
    this.saveMessage = null;
    this.syncResultsControls();
    try {
      const filename = await downloadScorecardPng(score);
      if (this.displayedScore() === score) {
        this.saveMessage = `Downloaded ${filename}`;
      }
    } catch (error) {
      if (this.displayedScore() === score) {
        this.saveMessage = `Could not save score: ${error instanceof Error ? error.message : String(error)}`;
      }
    } finally {
      this.savingScore = false;
      this.syncResultsControls();
    }
  }

  private recordCompletion(): void {
    if (!this.run.home || this.run.level + 1 !== LEVELS || this.completionRecorded) return;
    this.completionRecorded = true;
    const score = this.currentScore();
    if (!Number.isFinite(score.seconds) || score.seconds <= 0) return;

    const replay = this.recording.finish(this.run.ghostFrame());
    const fasterTime = this.personalBest === null || score.seconds < this.personalBest;
    const fasterReplay =
      replay !== null && (this.bestReplay === null || replay.seconds < this.bestReplay.seconds);
    if (fasterTime) {
      this.personalBest = score.seconds;
      this.bestScorecard = score;
    } else if (this.bestScorecard === null) {
      this.bestScorecard = score;
    }
    if (fasterReplay && replay !== null) {
      this.bestReplay = replay;
    }
    const unlocked = (this.achievements | this.run.completionAchievements(score)) & 0xffff;
    this.achievements = unlocked;
    const saved = saveCompletion(score, replay, unlocked);
    if (saved && saved.scorecard) {
      this.bestScorecard = saved.scorecard;
    }
  }

  private recordStunts(): void {
    const unlocked = (this.achievements | this.run.stunts.unlocked) & 0xffff;
    if (unlocked !== this.achievements) {
      this.achievements = unlocked;
      saveAchievements(unlocked);
    }
  }

  private syncScreenChrome(force: boolean): void {
    const showMenu = this.menuOpen;
    const inFlight = this.menuFlight !== null;
    const showResults = !showMenu && !inFlight && this.displayedScore() !== null;
    const showOptionsBtn = !this.optionsOpen && !showMenu && !inFlight;
    const hasScorecard = this.bestScorecard !== null;
    const key = `${showMenu ? 1 : 0}:${inFlight ? 1 : 0}:${showResults ? 1 : 0}:${showOptionsBtn ? 1 : 0}:${hasScorecard ? 1 : 0}`;
    if (!force && key === this.lastScreenKey) return;
    this.lastScreenKey = key;

    setVisible(this.backLink, showMenu);
    setVisible(this.menuActionsWrap, showMenu);
    setVisible(this.menuScorecardBtn, hasScorecard);
    setVisible(this.optionsBtn, showOptionsBtn);
    setVisible(this.resultsControlsEl, showResults);
    if (showResults) this.syncResultsControls();
  }

  private syncResultsControls(): void {
    this.downloadScoreBtn.disabled = this.savingScore;
    this.downloadScoreBtn.textContent = this.savingScore ? "Saving…" : "Download scorecard";
    this.resultsMessageEl.textContent = this.saveMessage ?? "";
  }

  private requestFrame(): void {
    this.needsRender = true;
    if (this.rafId === null) {
      this.rafId = requestAnimationFrame((nowMs) => this.tick(nowMs));
    }
  }

  private tick(nowMs: number): void {
    this.rafId = null;

    const active = this.windowFocused && !this.optionsOpen && !this.rotatePromptOpen;
    if (this.optionsOpen && this.musicPreview) {
      this.syncMusicPreview(this.windowFocused);
    } else {
      this.syncAudio(active);
    }

    const courseViewport = computeCourseViewport(this.viewportW, this.viewportH);
    const { viewW, cameraLead } = courseViewport;
    let keepAnimating = false;

    if (this.menuOpen || this.viewingScorecard !== null) {
      if (active && !this.achievementsOpen && !this.reducedMotion()) {
        if (this.lastFrameMs !== null) {
          const elapsed = Math.min(0.05, Math.max(0, (nowMs - this.lastFrameMs) / 1000));
          this.menuTime += elapsed;
          this.menuCamera = followAim(this.menuCamera, this.menuAim, elapsed);
        }
        this.lastFrameMs = nowMs;
        keepAnimating = true;
      } else {
        this.lastFrameMs = null;
      }
    } else {
      if (active && this.reducedMotion()) {
        if (this.menuFlight !== null) this.finishMenuFlight();
        if (this.slide !== null) this.finishSlide();
        if (this.arrival !== null) this.finishArrival();
      }
      if (active && (this.menuFlight !== null || this.slide !== null || this.arrival !== null)) {
        keepAnimating = true;
      }

      const elapsed = this.advanceActiveFrame(nowMs, active);
      if (elapsed !== null) {
        this.remainder += elapsed;
        const returnsBefore = this.run.returns;
        const coinsBefore = this.run.coinCount();
        const litBefore = this.run.litCount();
        const homeBefore = this.run.home;

        while (this.remainder >= STEP) {
          this.run.step();
          this.remainder -= STEP;
        }

        if (this.run.coinCount() > coinsBefore) this.audio.cue(CUE_COIN);
        if (this.run.litCount() > litBefore) this.audio.cue(CUE_SHUTTERS);
        if (this.run.returns > returnsBefore) this.audio.cue(CUE_RETURN);
        if (this.run.home !== homeBefore) {
          this.syncAudio(active);
        }

        this.recording.capture(this.run.ghostFrame());
        this.recordCompletion();
        this.recordStunts();

        const lastRoof = this.run.roofs[this.run.roofs.length - 1]!;
        const end = lastRoof.x + lastRoof.width;
        const target = clamp(this.run.x - viewW * cameraLead, 0, end - Math.min(viewW, end));
        if (this.run.returns !== returnsBefore || this.run.jumps === 0 || this.reducedMotion()) {
          this.camera = target;
        } else {
          this.camera += (target - this.camera) * (1 - Math.exp(-elapsed * 7));
        }

        if (!this.run.home || !this.reducedMotion()) {
          keepAnimating = true;
        }
      }

      if (
        active &&
        !this.reducedMotion() &&
        this.hudChangedMs !== null &&
        nowMs - this.hudChangedMs < 250
      ) {
        keepAnimating = true;
      }
    }

    this.syncScreenChrome(false);
    this.renderScene(courseViewport, nowMs);
    this.needsRender = false;

    if (keepAnimating || this.needsRender) {
      this.requestFrame();
    }
  }

  private advanceActiveFrame(nowMs: number, active: boolean): number | null {
    if (!active || this.optionsOpen) {
      this.lastFrameMs = null;
      return null;
    }
    const prev = this.lastFrameMs;
    this.lastFrameMs = nowMs;
    const elapsed = prev === null ? 0 : Math.max(0, (nowMs - prev) / 1000);

    if (this.menuFlight !== null) {
      if (this.menuFlight.advance(elapsed)) {
        this.finishMenuFlight();
      }
      return null;
    }
    if (this.slide !== null) {
      this.slide.time += Math.min(elapsed, 0.05);
      this.camera = this.slide.camera();
      if (this.slide.time >= LevelSlide.DURATION) {
        this.finishSlide();
      }
      return null;
    }
    if (this.arrival !== null) {
      const wasLit = this.run.lit[0] ?? false;
      this.arrival += Math.min(elapsed, 0.05);
      this.run.lit[0] = this.arrival >= ARRIVAL_LANDING;
      if (!wasLit && this.run.lit[0]) {
        this.audio.cue(CUE_SHUTTERS);
      }
      if (this.arrival >= ARRIVAL_DURATION) {
        this.finishArrival();
      }
      return null;
    }
    if (!this.started) {
      this.lastFrameMs = null;
      return null;
    }
    this.run.recordActiveTime(elapsed);
    return Math.min(elapsed, 0.05);
  }

  private renderScene(courseViewport: CourseViewport, nowMs: number): void {
    const { scale, viewW, viewH, verticalOffset } = courseViewport;
    const ctx = this.ctx;
    ctx.setTransform(this.dpr, 0, 0, this.dpr, 0, 0);

    if (this.rotatePromptOpen) {
      ctx.fillStyle = "#101322";
      ctx.fillRect(0, 0, this.viewportW, this.viewportH);
      return;
    }

    if (this.menuOpen) {
      const time = this.reducedMotion() ? 0 : this.menuTime;
      const aim = this.reducedMotion() ? defaultAim() : this.menuCamera;
      this.harborRenderer.renderMenu(
        ctx,
        this.viewportW,
        this.viewportH,
        time,
        aim,
        this.touchMode,
      );
      return;
    }

    if (this.menuFlight !== null && !this.menuFlight.revealsCourse()) {
      this.harborRenderer.renderFlight(
        ctx,
        this.viewportW,
        this.viewportH,
        this.menuFlight,
        this.menuTime,
        this.menuCamera,
        this.touchMode,
      );
      return;
    }

    const displayed = this.displayedScore();
    if (displayed !== null) {
      ctx.fillStyle = "#101322";
      ctx.fillRect(0, 0, this.viewportW, this.viewportH);
      const poster = this.posterMetrics();
      const sleepTime = this.reducedMotion()
        ? 1.2
        : this.viewingScorecard !== null
          ? this.menuTime
          : this.run.time;
      ctx.save();
      ctx.beginPath();
      ctx.roundRect(poster.x, poster.y, poster.width, poster.height, 8 * poster.scale);
      ctx.clip();
      ctx.translate(poster.x, poster.y);
      renderPosterToCanvas(ctx, displayed, poster.scale, sleepTime);
      ctx.restore();
      return;
    }

    const drift = this.reducedMotion() ? 1.2 : this.run.time + (this.slide ? this.slide.time : 0);

    const ghost =
      this.preferences.ghost &&
      this.arrival === null &&
      this.slide === null &&
      this.ghostReplay !== null
        ? frameAt(this.ghostReplay, this.run.elapsed)
        : null;

    const gap = ghost !== null ? ghostGap(this.run, ghost, this.camera, viewW) : null;

    this.courseRenderer.render(ctx, {
      run: this.run,
      slide: this.slide,
      arrival: this.arrival,
      camera: this.camera,
      w: viewW,
      h: viewH,
      verticalOffset,
      scale,
      drift,
      reducedMotion: this.reducedMotion(),
      ghost,
      showGhost: ghost !== null && gap === null,
    });

    let nameFirst = !this.started || this.run.home || this.optionsOpen || !this.windowFocused;
    let clockOpacity = 1;
    if (!nameFirst && !this.reducedMotion() && this.hudChangedMs !== null) {
      const t = Math.min(1, (nowMs - this.hudChangedMs) / 240);
      nameFirst = t < 0.5;
      clockOpacity = Math.abs(2 * t - 1);
    }

    const touchLayout = this.touchMode ? this.currentTouchLayout() : null;
    const touchControls: TouchControlState | null =
      touchLayout !== null
        ? {
            layout: touchLayout,
            joyActive: this.movePointerId !== null,
            joyOriginX: this.movePointerId !== null ? this.joyOriginX : touchLayout.joyBaseX,
            joyOriginY: this.movePointerId !== null ? this.joyOriginY : touchLayout.joyBaseY,
            knobX: this.movePointerId !== null ? this.joyKnobX : touchLayout.joyBaseX,
            knobY: this.movePointerId !== null ? this.joyKnobY : touchLayout.joyBaseY,
            direction: this.moveDirection,
            jumpPressed: this.jumpPointerId !== null,
            doorwayReady: this.run.home,
          }
        : null;

    drawCourseHud(ctx, {
      viewportW: this.viewportW,
      viewportH: this.viewportH,
      score: this.run.score(),
      level: this.run.level,
      nameFirst,
      clockOpacity,
      gap,
      showArrivalCaption: this.run.level === 0 && !this.started,
      showDoorwayPrompt: this.run.home,
      touchMode: this.touchMode,
      touchControls,
      safeTop: this.safeTop,
      safeLeft: this.safeLeft,
      safeRight: this.safeRight,
      safeBottom: this.safeBottom,
    });

    if (this.menuFlight !== null) {
      const veil = this.menuFlight.veilOpacity();
      if (veil > 0) {
        ctx.fillStyle = "#000000";
        ctx.globalAlpha = veil;
        ctx.fillRect(0, 0, this.viewportW, this.viewportH);
        ctx.globalAlpha = 1;
      }
    }
  }
}

const mount = document.getElementById("rooftops-root");
if (mount) {
  new RooftopsGame(mount);
}
