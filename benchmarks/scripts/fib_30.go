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
	num := 30
	fmt.Println(fibonacci(num))

	elapsed := time.Since(start)
	// Output in milliseconds for consistency with larger benchmarks
	fmt.Printf("%.2f\n", float64(elapsed.Nanoseconds())/1_000_000)
}
