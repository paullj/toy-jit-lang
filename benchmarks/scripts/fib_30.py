import time


def fibonacci(n: int) -> int:
    if n == 0:
        return 0
    elif n == 1:
        return 1
    else:
        return fibonacci(n - 1) + fibonacci(n - 2)


start_time = time.perf_counter_ns()
num = 30
print(fibonacci(num))

end_time = time.perf_counter_ns()
# Output in milliseconds for consistency with larger benchmarks
print(f"{(end_time - start_time) / 1_000_000:.2f}")
