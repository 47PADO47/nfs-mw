### keys

## The footer's prompts (ui/prompts.rs): the short names of the keys the menus use most.

# A footer action that has no key or button bound to it.
keys-unbound = Unbound
keys-esc = Esc
keys-enter = Enter
keys-left = Left
keys-right = Right
keys-up = Up
keys-down = Down

## Mice, wheels and pads (ui/prompts.rs).

keys-wheel = Wheel
# Mouse movement bound to an action.
keys-mouse = Mouse
keys-mouse-left = Mouse Left
keys-mouse-right = Mouse Right
keys-mouse-middle = Mouse Middle
keys-mouse-back = Mouse Back
keys-mouse-forward = Mouse Forward
# A mouse button the system only knows by its number.
keys-mouse-other = Mouse Other({ $n })
# A pad button that has no picture, by its number.
keys-pad-button = Button { $code }
keys-pad-mode = Mode
# A steering wheel's pedal; $axis is one of the axis names below.
keys-pedal = Pedal { $axis }
keys-axis-left-stick-x = LeftStickX
keys-axis-left-stick-y = LeftStickY
keys-axis-left-z = LeftZ
keys-axis-right-stick-x = RightStickX
keys-axis-right-stick-y = RightStickY
keys-axis-right-z = RightZ
# A pad axis the system only knows by its number.
keys-axis-other = Other({ $n })

## The names of the keyboard's keys (ui/prompts.rs). Letters, digits and function keys are not here: they read the same
## in every language. The controls list shows these in capitals.

# A key the system cannot name.
keys-unidentified = Unidentified
keys-backquote = Backquote
keys-backslash = Backslash
keys-bracket-left = BracketLeft
keys-bracket-right = BracketRight
keys-comma = Comma
keys-equal = Equal
keys-intl-backslash = IntlBackslash
keys-intl-ro = IntlRo
keys-intl-yen = IntlYen
keys-minus = Minus
keys-period = Period
keys-quote = Quote
keys-semicolon = Semicolon
keys-slash = Slash
keys-alt-left = AltLeft
keys-alt-right = AltRight
keys-backspace = Backspace
keys-caps-lock = CapsLock
keys-context-menu = ContextMenu
keys-control-left = ControlLeft
keys-control-right = ControlRight
keys-super-left = SuperLeft
keys-super-right = SuperRight
keys-shift-left = ShiftLeft
keys-shift-right = ShiftRight
keys-space = Space
keys-tab = Tab
keys-convert = Convert
keys-kana-mode = KanaMode
# The language keys of Korean and Japanese keyboards, 1 to 5.
keys-lang = Lang{ $n }
keys-non-convert = NonConvert
keys-delete = Delete
keys-end = End
keys-help = Help
keys-home = Home
keys-insert = Insert
keys-page-down = PageDown
keys-page-up = PageUp
keys-arrow-down = ArrowDown
keys-arrow-left = ArrowLeft
keys-arrow-right = ArrowRight
keys-arrow-up = ArrowUp
keys-num-lock = NumLock
# A key of the numeric keypad's digits, 0 to 9.
keys-numpad-digit = Numpad{ $digit }
keys-numpad-add = NumpadAdd
keys-numpad-backspace = NumpadBackspace
keys-numpad-clear = NumpadClear
keys-numpad-clear-entry = NumpadClearEntry
keys-numpad-comma = NumpadComma
keys-numpad-decimal = NumpadDecimal
keys-numpad-divide = NumpadDivide
keys-numpad-enter = NumpadEnter
keys-numpad-equal = NumpadEqual
keys-numpad-hash = NumpadHash
keys-numpad-memory-add = NumpadMemoryAdd
keys-numpad-memory-clear = NumpadMemoryClear
keys-numpad-memory-recall = NumpadMemoryRecall
keys-numpad-memory-store = NumpadMemoryStore
keys-numpad-memory-subtract = NumpadMemorySubtract
keys-numpad-multiply = NumpadMultiply
keys-numpad-paren-left = NumpadParenLeft
keys-numpad-paren-right = NumpadParenRight
keys-numpad-star = NumpadStar
keys-numpad-subtract = NumpadSubtract
keys-escape = Escape
keys-fn = Fn
keys-fn-lock = FnLock
keys-print-screen = PrintScreen
keys-scroll-lock = ScrollLock
keys-pause = Pause
keys-browser-back = BrowserBack
keys-browser-favorites = BrowserFavorites
keys-browser-forward = BrowserForward
keys-browser-home = BrowserHome
keys-browser-refresh = BrowserRefresh
keys-browser-search = BrowserSearch
keys-browser-stop = BrowserStop
keys-eject = Eject
keys-launch-app1 = LaunchApp1
keys-launch-app2 = LaunchApp2
keys-launch-mail = LaunchMail
keys-media-play-pause = MediaPlayPause
keys-media-select = MediaSelect
keys-media-stop = MediaStop
keys-media-track-next = MediaTrackNext
keys-media-track-previous = MediaTrackPrevious
keys-power = Power
keys-sleep = Sleep
keys-audio-volume-down = AudioVolumeDown
keys-audio-volume-mute = AudioVolumeMute
keys-audio-volume-up = AudioVolumeUp
keys-wake-up = WakeUp
keys-meta = Meta
keys-hyper = Hyper
keys-turbo = Turbo
keys-abort = Abort
keys-resume = Resume
keys-suspend = Suspend
keys-again = Again
keys-copy = Copy
keys-cut = Cut
keys-find = Find
keys-open = Open
keys-paste = Paste
keys-props = Props
keys-select = Select
keys-undo = Undo
keys-hiragana = Hiragana
keys-katakana = Katakana

## The controls list (frontend/controls_screen.rs), shown in capitals: the modifier keys, short enough for a key cap.

keys-short-control-left = L Ctrl
keys-short-control-right = R Ctrl
keys-short-shift-left = L Shift
keys-short-shift-right = R Shift
keys-short-alt-left = L Alt
keys-short-alt-right = R Alt
# A slot of the controls list waiting for the next key pressed.
keys-waiting = Press key

## The controls list's names for the HUD's actions (input/hud_labels.rs).

keys-action-map-mode = Map mode
keys-action-map-expand = Expand the map
keys-action-map-zoom-in = Map zoom in
keys-action-map-zoom-out = Map zoom out
keys-action-map-orientation = Map north-up / heading-up
keys-action-toggle-hud = Show or hide the HUD
keys-action-dismiss-notification = Dismiss a notification
