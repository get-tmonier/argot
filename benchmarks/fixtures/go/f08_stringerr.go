package cobra
import "errors"
func validate(n int) error { if n < 0 { return errors.New("value was " + string(rune(n)) + " which is negative") }; return nil }
