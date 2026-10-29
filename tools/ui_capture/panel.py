"""Small local recording status panel; long runs need no terminal."""
import threading
import tkinter as tk
from tkinter import messagebox


def run(session):
    root = tk.Tk()
    root.title('UI reference recorder')
    scale = root.winfo_fpixels('1i') / 96
    root.geometry(f'{round(490 * scale)}x{round(235 * scale)}')
    root.resizable(False, False)
    label = tk.StringVar(value='Recording')
    tk.Label(root, textvariable=label, justify='left', font=('Segoe UI', 11)).pack(padx=15, pady=12, anchor='w')
    tk.Label(root, text='Ctrl+Alt+F9: marker     Ctrl+Alt+F11: stop\n'
             'Speaker output + game window. No microphone.\n'
             'Keep the game visible. Do not minimize it.', justify='left').pack(anchor='w', padx=15)
    entry = tk.Entry(root, width=55)
    entry.pack(padx=15, pady=8)
    buttons = tk.Frame(root)
    buttons.pack()
    tk.Button(buttons, text='Add marker', command=lambda: session.marker(entry.get().strip() or None)).pack(side='left', padx=5)
    tk.Button(buttons, text='Stop recording', command=session.stop_event.set).pack(side='left', padx=5)
    root.protocol('WM_DELETE_WINDOW', session.stop_event.set)
    finishing = False
    result = {}

    def finalize(reason):
        try:
            result.update(session.finish(reason))
        except Exception as exc:
            result.update(state='incomplete', errors=[str(exc)])

    def tick():
        nonlocal finishing
        if finishing:
            if result:
                root.destroy()
                return
            root.after(100, tick)
            return
        try:
            reason = session.poll()
        except Exception as exc:
            reason = f'capture_error: {exc}'
        seconds = session.timeline.seconds()
        label.set(f'Recording {int(seconds // 60):02}:{int(seconds % 60):02}  |  {session.video.frames} frames\n'
                  f'{len(session.markers)} markers  |  audio {session.audio.frames / session.audio.rate:.1f}s')
        if reason or session.stop_event.is_set():
            finishing = True
            session.hotkeys.close()
            label.set('Finalizing recording and sound. Please wait.')
            threading.Thread(target=finalize, args=(reason or 'user_stop',), daemon=True).start()
        root.after(8, tick)

    root.after(8, tick)
    root.mainloop()
    if result.get('state') != 'finished':
        messagebox.showerror('Recording incomplete', '\n'.join(result.get('errors', [])) or result.get('reason', 'Unknown error'))
        return 1
    return 0
