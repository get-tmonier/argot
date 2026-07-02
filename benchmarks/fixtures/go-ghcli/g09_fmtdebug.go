package cmd

import "fmt"

func traceExec(args []string) {
	fmt.Println(">>> executing with args:", args)
	fmt.Println(">>> arg count:", len(args))
	fmt.Println(">>> done")
}
