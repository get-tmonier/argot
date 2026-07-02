package hugolib
import "errors"
func checkDepth(d int) error { if d > 10 { return errors.New("recursion depth " + string(rune(d)) + " exceeds limit") }; return nil }
