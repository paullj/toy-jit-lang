package main

import (
	"fmt"
	"time"
)

func fibonacci(n int) int {
	if n == 0 {
		return 0
	} else if n == 1 {
		return 1
	} else {
		return fibonacci(n-1) + fibonacci(n-2)
	}
}

func main() {
	start := time.Now()
	num := 20
	fmt.Println(fibonacci(num))

	elapsed := time.Since(start)
	fmt.Printf("Execution time: %.2f ms\n", float64(elapsed.Nanoseconds())/1_000_000)
	fmt.Printf("Execution time in us: %.2f µs\n", float64(elapsed.Nanoseconds())/1_000)
}
