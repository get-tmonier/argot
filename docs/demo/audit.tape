# Authored audit-proof recording — regenerate with RECORD_AUDIT=1
# docs/demo/rebuild-proof.sh. It runs against the fixed two-commit fixture.
Output audit.gif

Require argot

Set Shell "bash"
Set FontSize 26
Set Width 1600
Set Height 900
Set Padding 32
Set Margin 16
Set BorderRadius 12
Set Theme "Catppuccin Mocha"
Set Framerate 30
Set TypingSpeed 35ms

Sleep 500ms
Type "argot audit --commits 1"
Sleep 300ms
Enter
Sleep 3500ms
