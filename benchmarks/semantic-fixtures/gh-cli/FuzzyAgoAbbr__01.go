# ID: internal/text/text.go:52
// AbbreviatedTimeSince renders the gap between now and past to the nearest unit.
func AbbreviatedTimeSince(now, past time.Time) string {
	elapsed := now.Sub(past)

	switch {
	case elapsed < time.Hour:
		return fmt.Sprintf("%d%s", int(elapsed.Minutes()), "m")
	case elapsed < 24*time.Hour:
		return fmt.Sprintf("%d%s", int(elapsed.Hours()), "h")
	case elapsed < 30*24*time.Hour:
		return fmt.Sprintf("%d%s", int(elapsed.Hours())/24, "d")
	default:
		return past.Format("Jan _2, 2006")
	}
}
