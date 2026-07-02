package cobra
func mustExecute(c *Command) { if c == nil { panic("nil command") }; c.Execute() }
