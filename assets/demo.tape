# Run from the repository root with the release binary first on PATH:
# cargo build --release && PATH=$PWD/target/release:$PATH vhs assets/demo.tape

Output assets/demo.gif
Require retiretui

Set Shell bash
Set FontSize 14
Set Width 1280
Set Height 720
Set Padding 10
Set Framerate 12
Set TypingSpeed 60ms

Hide
Type "clear"
Enter
Show

Type "retiretui tui examples/"
Sleep 500ms
Enter
Sleep 2s
Type "early-retiree.toml"
Sleep 1s
Enter
Sleep 3s

Right@400ms 4
Sleep 2s

Type "2"
Sleep 1s
Down@400ms 5
Sleep 2s

Type "4"
Sleep 1s
Down@600ms 2
Sleep 6s

Type "5"
Sleep 1500ms
Right
Sleep 1s
Enter
Sleep 3s
Escape
Sleep 1s

Type "q"
Sleep 500ms
