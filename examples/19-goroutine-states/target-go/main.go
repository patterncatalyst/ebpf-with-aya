// A small Go program that creates lots of goroutine state churn: goroutines
// that run, block on channels, sleep, and exit — so runtime.casgstatus fires
// across many states for the tracer to observe.
package main

import (
	"fmt"
	"os"
	"sync"
	"time"
)

func worker(id int, ch chan int, wg *sync.WaitGroup) {
	defer wg.Done()
	for v := range ch { // blocks (waiting) then runs
		_ = v * v
		time.Sleep(2 * time.Millisecond) // sleep -> waiting -> runnable -> running
	}
}

func main() {
	fmt.Printf("target-go pid %d — churning goroutine states\n", os.Getpid())
	for {
		ch := make(chan int)
		var wg sync.WaitGroup
		for i := 0; i < 8; i++ {
			wg.Add(1)
			go worker(i, ch, &wg) // new goroutine: idle -> runnable -> running
		}
		for i := 0; i < 200; i++ {
			ch <- i
		}
		close(ch)
		wg.Wait()
		time.Sleep(200 * time.Millisecond)
	}
}
