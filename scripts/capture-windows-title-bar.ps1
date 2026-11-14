param(
    [Parameter(Mandatory)][string] $Executable,
    [Parameter(Mandatory)][string] $OutputDirectory
)

$ErrorActionPreference = 'Stop'
$exe = (Resolve-Path $Executable).Path
New-Item -ItemType Directory -Force $OutputDirectory | Out-Null
$report = Join-Path $OutputDirectory 'interactions.txt'
Add-Type -AssemblyName System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;

public static class CaptionTest {
    [StructLayout(LayoutKind.Sequential)]
    public struct Rect {
        public int Left, Top, Right, Bottom;
        public int Width => Right - Left;
        public int Height => Bottom - Top;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct Point {
        public int X, Y;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct GuiThread {
        public uint Size, Flags;
        public IntPtr Active, Focus, Capture, MenuOwner, MoveSize, Caret;
        public Rect CaretRect;
    }

    [DllImport("user32.dll")] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] private static extern IntPtr FindWindow(string cls, string title);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint owner);
    [DllImport("user32.dll")] public static extern bool GetGUIThreadInfo(uint thread, ref GuiThread info);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern bool IsWindow(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern bool IsZoomed(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern bool ShowWindowAsync(IntPtr hwnd, int command);
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr hwnd, out Rect rect);
    [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr hwnd, ref Point point);
    [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr hwnd);
    [DllImport("dwmapi.dll")] public static extern int DwmGetWindowAttribute(IntPtr hwnd, int key, out Rect rect, int size);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] private static extern bool GetCursorPos(out Point point);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] private static extern IntPtr SendMessageTimeout(
        IntPtr hwnd, uint message, UIntPtr wparam, IntPtr lparam, uint flags, uint milliseconds, out IntPtr result);
    [DllImport("user32.dll")] private static extern void mouse_event(uint flags, uint x, uint y, uint data, UIntPtr extra);
    [DllImport("user32.dll")] private static extern void keybd_event(byte key, byte scan, uint flags, UIntPtr extra);

    public static IntPtr FindByTitle(string title) => FindWindow(null, title);
    public static void Mouse(uint flags) => mouse_event(flags, 0, 0, 0, UIntPtr.Zero);
    public static void Key(byte key, uint flags) => keybd_event(key, 0, flags, UIntPtr.Zero);

    public static int HitAtCursor(IntPtr window) {
        Point cursor;
        if (!GetCursorPos(out cursor)) throw new Exception("Cannot read the cursor position");
        var coords = new IntPtr((cursor.Y << 16) | (cursor.X & 0xffff));
        IntPtr hit;
        if (SendMessageTimeout(window, 0x84, UIntPtr.Zero, coords, 2, 1000, out hit) == IntPtr.Zero) {
            throw new Exception("WM_NCHITTEST timed out");
        }
        return hit.ToInt32();
    }

    public static Rect Client(IntPtr window) {
        Rect rect;
        var origin = new Point();
        if (!GetClientRect(window, out rect) || !ClientToScreen(window, ref origin)) {
            throw new Exception("Cannot read client bounds");
        }
        return new Rect { Left = origin.X, Top = origin.Y, Right = origin.X + rect.Right, Bottom = origin.Y + rect.Bottom };
    }

    public static Rect Frame(IntPtr window) {
        Rect rect;
        if (DwmGetWindowAttribute(window, 9, out rect, Marshal.SizeOf<Rect>()) != 0) {
            throw new Exception("Cannot read the DWM frame bounds");
        }
        return rect;
    }

    public static int MenuFlags(IntPtr window) {
        uint owner;
        uint thread = GetWindowThreadProcessId(window, out owner);
        var info = new GuiThread { Size = (uint)Marshal.SizeOf<GuiThread>() };
        if (!GetGUIThreadInfo(thread, ref info)) {
            throw new Exception("Cannot read the menu state");
        }
        return (int)info.Flags;
    }
}
'@

function Write-Result([string] $Message) {
    Write-Host $Message
    Add-Content -Path $report -Value $Message
}

function Move-Pointer([int] $X, [int] $Y) {
    if (-not [CaptionTest]::SetCursorPos($X, $Y)) { throw 'SetCursorPos failed' }
    Start-Sleep -Milliseconds 140
}

function Click-Pointer([switch] $Right) {
    $down, $up = if ($Right) { 8, 16 } else { 2, 4 }
    [CaptionTest]::Mouse($down)
    Start-Sleep -Milliseconds 45
    [CaptionTest]::Mouse($up)
}

function Press-Key([byte] $Key, [switch] $Alt) {
    if ($Alt) { [CaptionTest]::Key(0x12, 0) }
    [CaptionTest]::Key($Key, 0)
    [CaptionTest]::Key($Key, 2)
    if ($Alt) { [CaptionTest]::Key(0x12, 2) }
}

function Move-ToCaption([IntPtr] $Window, [int] $FromRight) {
    $client = [CaptionTest]::Client($Window)
    $scale = [CaptionTest]::GetDpiForWindow($Window) / 96.0
    Move-Pointer ($client.Right - $FromRight * $scale) ($client.Top + 20 * $scale)
    Move-Pointer ($client.Right - $FromRight * $scale - 1) ($client.Top + 20 * $scale)
}

function Move-ToDragArea([IntPtr] $Window) {
    $client = [CaptionTest]::Client($Window)
    $scale = [CaptionTest]::GetDpiForWindow($Window) / 96.0
    Move-Pointer ($client.Left + 100 * $scale) ($client.Top + 20 * $scale)
    Move-Pointer ($client.Left + 101 * $scale) ($client.Top + 20 * $scale)
}

function Capture-Window([IntPtr] $Window, [string] $Name) {
    $null = [CaptionTest]::SetForegroundWindow($Window)
    $client = [CaptionTest]::Client($Window)
    Move-Pointer ($client.Left + 12) ($client.Bottom - 22)
    Start-Sleep -Milliseconds 700
    if ([CaptionTest]::GetForegroundWindow() -ne $Window) { throw "Window for $Name is not foreground" }
    $frame = [CaptionTest]::Frame($Window)
    $bitmap = [System.Drawing.Bitmap]::new($frame.Width, $frame.Height)
    try {
        $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
        try {
            $graphics.CopyFromScreen($frame.Left, $frame.Top, 0, 0, $bitmap.Size)
        } finally {
            $graphics.Dispose()
        }
        $colors = @(
            for ($x = 1; $x -lt 16; $x++) {
                for ($y = 1; $y -lt 16; $y++) {
                    $bitmap.GetPixel([int]($x * $frame.Width / 16), [int]($y * $frame.Height / 16)).ToArgb()
                }
            }
        ) | Sort-Object -Unique
        $bitmap.Save((Join-Path $OutputDirectory "$Name.png"), [System.Drawing.Imaging.ImageFormat]::Png)
        if ($colors.Count -lt 6) { throw "The captured desktop looks blank for $Name" }
        if ($Name -eq 'attachment-editor') {
            $caption = $bitmap.GetPixel([int]($frame.Width / 2), 8)
            if ($caption.GetBrightness() -gt 0.45) { throw 'The native editor title bar still looks light' }
            Write-Result "PASS Dark native editor title bar ($($caption.R), $($caption.G), $($caption.B))"
        }
        Write-Result "Captured $Name ($($frame.Width)x$($frame.Height), $([CaptionTest]::GetDpiForWindow($Window)) DPI)."
    } finally {
        $bitmap.Dispose()
    }
}

function Check-Interaction([string] $Name, [scriptblock] $Action) {
    try {
        & $Action
        Write-Result "PASS $Name"
    } catch {
        Write-Result "FAIL $Name - $_"
        $script:failed = $true
    }
}

function Check-Caption([IntPtr] $Window) {
    Check-Interaction 'Maximize and restore buttons' {
        Move-ToCaption $Window 51
        $hit = [CaptionTest]::HitAtCursor($Window)
        if ($hit -ne 9) { throw "Expected HTMAXBUTTON, received $hit" }
        Click-Pointer
        Start-Sleep -Milliseconds 650
        if (-not [CaptionTest]::IsZoomed($Window)) { throw 'Maximize did not take effect' }
        Capture-Window $Window 'uploader-maximized'
        Move-ToCaption $Window 51
        Click-Pointer
        Start-Sleep -Milliseconds 650
        if ([CaptionTest]::IsZoomed($Window)) { throw 'Restore did not take effect' }
    }
    Check-Interaction 'Minimize button' {
        Move-ToCaption $Window 85
        Click-Pointer
        Start-Sleep -Milliseconds 450
        if (-not [CaptionTest]::IsIconic($Window)) { throw 'Window stayed visible' }
        $null = [CaptionTest]::ShowWindowAsync($Window, 9)
        Start-Sleep -Milliseconds 500
        $null = [CaptionTest]::SetForegroundWindow($Window)
    }
    Check-Interaction 'Title bar double-click' {
        Move-ToDragArea $Window
        Click-Pointer
        Click-Pointer
        Start-Sleep -Milliseconds 650
        if (-not [CaptionTest]::IsZoomed($Window)) { throw 'Double-click did not maximize' }
        Move-ToDragArea $Window
        Click-Pointer
        Click-Pointer
        Start-Sleep -Milliseconds 650
        if ([CaptionTest]::IsZoomed($Window)) { throw 'Double-click did not restore' }
    }
    Check-Interaction 'Title bar drag' {
        $before = [CaptionTest]::Client($Window)
        Move-ToDragArea $Window
        [CaptionTest]::Mouse(2)
        Start-Sleep -Milliseconds 200
        Move-Pointer ($before.Left + 71) ($before.Top + 45)
        [CaptionTest]::Mouse(4)
        Start-Sleep -Milliseconds 550
        $after = [CaptionTest]::Client($Window)
        if ($before.Left -eq $after.Left -and $before.Top -eq $after.Top) { throw 'Window did not move' }
    }
    Check-Interaction 'Top edge resize' {
        $before = [CaptionTest]::Client($Window)
        $x = $before.Left + 200
        Move-Pointer $x ($before.Top + 2)
        [CaptionTest]::Mouse(2)
        Start-Sleep -Milliseconds 200
        Move-Pointer $x ($before.Top - 22)
        [CaptionTest]::Mouse(4)
        Start-Sleep -Milliseconds 550
        $after = [CaptionTest]::Client($Window)
        if ($before.Height -eq $after.Height) { throw 'Window height did not change' }
    }
    Check-Interaction 'Alt+Space system menu' {
        Press-Key 0x20 -Alt
        Start-Sleep -Milliseconds 400
        $inMenu = [CaptionTest]::MenuFlags($Window) -band 8
        Press-Key 0x1b
        Start-Sleep -Milliseconds 500
        if (-not $inMenu) { throw 'Alt+Space did not open the system menu' }
    }
}

$null = [CaptionTest]::SetThreadDpiAwarenessContext([IntPtr](-4))
$script:failed = $false
Write-Result "Runner session $((Get-Process -Id $PID).SessionId)"
foreach ($scenario in 'uploader', 'caption-context-menu', 'caption-close', 'compact', 'sidebar-collapsed', 'preferences', 'attachment-editor') {
    $env:GATTO_WINDOWS_CAPTURE = $scenario
    $title = if ($scenario -eq 'attachment-editor') { 'interface-review.png — Gatto preview' } else { 'Gatto' }
    $app = Start-Process $exe -PassThru
    try {
        $window = [IntPtr]::Zero
        for ($attempt = 0; $attempt -lt 40; $attempt++) {
            Start-Sleep -Milliseconds 650
            if ($app.HasExited) { throw "Gatto exited with code $($app.ExitCode)" }
            $candidate = [CaptionTest]::FindByTitle($title)
            if ($candidate -ne [IntPtr]::Zero) {
                [uint32] $owner = 0
                $null = [CaptionTest]::GetWindowThreadProcessId($candidate, [ref]$owner)
                if ($owner -eq $app.Id) {
                    $window = $candidate
                    break
                }
            }
        }
        if ($window -eq [IntPtr]::Zero) { throw "Timed out opening $title" }
        if ($scenario -notlike 'caption-*') { Capture-Window $window $scenario }
        if ($scenario -eq 'uploader') { Check-Caption $window }
        if ($scenario -eq 'compact') {
            Check-Interaction 'Repository picker is clickable beside the caption' {
                $client = [CaptionTest]::Client($window)
                Move-Pointer ($client.Right - 190) ($client.Top + 26)
                Move-Pointer ($client.Right - 191) ($client.Top + 26)
                $hit = [CaptionTest]::HitAtCursor($window)
                if ($hit -ne 1) { throw "Expected HTCLIENT, received $hit" }
                Click-Pointer
                Capture-Window $window 'repository-picker'
            }
        }
        if ($scenario -eq 'caption-context-menu') {
            Check-Interaction 'Caption context menu' {
                Start-Sleep -Milliseconds 500
                Move-ToDragArea $window
                $hit = [CaptionTest]::HitAtCursor($window)
                if ($hit -ne 2) { throw "Expected HTCAPTION, received $hit" }
                Click-Pointer -Right
                Start-Sleep -Milliseconds 600
                $flags = [CaptionTest]::MenuFlags($window)
                Write-Result "Caption right-click menu flags $flags"
                if (-not ($flags -band 0x1c)) { throw 'Right-click did not open a menu' }
                Press-Key 0x1b
            }
        }
        if ($scenario -eq 'caption-close') {
            Check-Interaction 'Caption close button' {
                Start-Sleep -Milliseconds 500
                Move-ToCaption $window 17
                $hit = [CaptionTest]::HitAtCursor($window)
                if ($hit -ne 20) { throw "Expected HTCLOSE, received $hit" }
                Click-Pointer
                Start-Sleep -Milliseconds 900
                if ([CaptionTest]::IsWindow($window)) { throw 'Close did not destroy the window' }
            }
        }
        if ($scenario -eq 'preferences') {
            Check-Interaction 'Alt+F4 close' {
                Press-Key 0x73 -Alt
                Start-Sleep -Milliseconds 650
                if ([CaptionTest]::IsWindow($window)) { throw 'Alt+F4 did not destroy the window' }
            }
        }
    } catch {
        Write-Result "FAIL $scenario window - $_"
        $script:failed = $true
    } finally {
        if (-not $app.HasExited) {
            Stop-Process -Id $app.Id -Force
            $app.WaitForExit()
        }
    }
}

if ($failed) { throw 'One or more Windows capture checks failed. See the uploaded report.' }
