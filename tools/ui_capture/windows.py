"""Read-only window and foreground input observation, plus recorder hotkeys."""
import ctypes as c
from ctypes import wintypes as w
from pathlib import Path

u = c.WinDLL('user32', use_last_error=True)
k = c.WinDLL('kernel32', use_last_error=True)
u.SetProcessDpiAwarenessContext.argtypes = [w.HANDLE]
u.SetProcessDpiAwarenessContext(w.HANDLE(-4))
u.GetForegroundWindow.restype = w.HWND
u.IsWindow.argtypes = [w.HWND]
u.IsWindowVisible.argtypes = [w.HWND]
u.GetWindowThreadProcessId.argtypes = [w.HWND, c.POINTER(w.DWORD)]
u.GetWindowTextW.argtypes = [w.HWND, w.LPWSTR, c.c_int]
u.GetClientRect.argtypes = [w.HWND, c.POINTER(w.RECT)]
u.ScreenToClient.argtypes = [w.HWND, c.POINTER(w.POINT)]
k.OpenProcess.argtypes = [w.DWORD, w.BOOL, w.DWORD]
k.OpenProcess.restype = w.HANDLE
k.QueryFullProcessImageNameW.argtypes = [w.HANDLE, w.DWORD, w.LPWSTR, c.POINTER(w.DWORD)]
k.CloseHandle.argtypes = [w.HANDLE]


def executable(pid):
    handle = k.OpenProcess(0x1000, False, pid)
    if not handle:
        return None
    try:
        size = w.DWORD(32768)
        text = c.create_unicode_buffer(size.value)
        if k.QueryFullProcessImageNameW(handle, 0, text, c.byref(size)):
            return Path(text.value)
        return None
    finally:
        k.CloseHandle(handle)


def windows(exe):
    result = []
    expected = Path(exe).resolve()
    callback_type = c.WINFUNCTYPE(w.BOOL, w.HWND, w.LPARAM)

    @callback_type
    def visit(hwnd, _):
        if not u.IsWindowVisible(hwnd):
            return True
        pid = w.DWORD()
        u.GetWindowThreadProcessId(hwnd, c.byref(pid))
        path = executable(pid.value)
        if path is None or path.resolve() != expected:
            return True
        rect = w.RECT()
        u.GetClientRect(hwnd, c.byref(rect))
        if rect.right <= 0 or rect.bottom <= 0:
            return True
        title = c.create_unicode_buffer(1024)
        u.GetWindowTextW(hwnd, title, len(title))
        result.append(dict(hwnd=int(hwnd), pid=pid.value, executable=str(path), title=title.value,
                           width=rect.right, height=rect.bottom))
        return True

    u.EnumWindows(visit, 0)
    return result


class Pad(c.Structure):
    _fields_ = [('buttons', w.WORD), ('left_trigger', c.c_ubyte), ('right_trigger', c.c_ubyte),
                ('lx', c.c_short), ('ly', c.c_short), ('rx', c.c_short), ('ry', c.c_short)]


class PadState(c.Structure):
    _fields_ = [('packet', w.DWORD), ('pad', Pad)]


class Inputs:
    def __init__(self, hwnd, timeline):
        self.hwnd, self.timeline = hwnd, timeline
        self.last = None
        self.last_pads = {}
        self.pad = c.WinDLL('xinput1_4').XInputGetState
        self.pad.argtypes = [w.DWORD, c.POINTER(PadState)]
        self.previous_position = None
        self.next_pointer = 0

    def sample(self):
        foreground = u.GetForegroundWindow() == self.hwnd
        keys = []
        if foreground:
            keys = [vk for vk in range(1, 255) if u.GetAsyncKeyState(vk) & 0x8000]
        state = (foreground, tuple(keys))
        if state != self.last:
            self.timeline.emit('input', foreground=foreground, held_vk=keys)
            self.last = state
        if not foreground:
            self.last_pads.clear()
            return
        for index in range(4):
            raw = PadState()
            present = self.pad(index, c.byref(raw)) == 0
            value = None
            if present:
                value = {name: getattr(raw.pad, name) for name, _ in Pad._fields_}
            if self.last_pads.get(index, 'unknown') != value:
                self.timeline.emit('xinput', index=index, state=value)
                self.last_pads[index] = value
        now = self.timeline.seconds()
        if now < self.next_pointer:
            return
        self.next_pointer = now + 1 / 30
        point = w.POINT()
        u.GetCursorPos(c.byref(point))
        u.ScreenToClient(self.hwnd, c.byref(point))
        position = (point.x, point.y)
        if position != self.previous_position:
            self.timeline.emit('pointer', client_xy=position)
            self.previous_position = position


class Hotkeys:
    def __init__(self):
        self.ids = []
        for identity, vk in [(1, 0x78), (2, 0x7a)]:  # Ctrl+Alt+F9 / F11; F10 is deliberately unused.
            if not u.RegisterHotKey(None, identity, 0x4003, vk):
                self.close()
                raise RuntimeError(f'Cannot register recorder hotkey {identity}; another app owns it')
            self.ids.append(identity)

    def poll(self):
        message = w.MSG()
        while u.PeekMessageW(c.byref(message), None, 0x312, 0x312, 1):
            yield int(message.wParam)

    def close(self):
        for identity in self.ids:
            u.UnregisterHotKey(None, identity)
        self.ids.clear()
