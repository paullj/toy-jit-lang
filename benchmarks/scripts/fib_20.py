import time


def fibonacci(n: int) -> int:
    if n == 0:
        return 0
    elif n == 1:
        return 1
    else:
        return fibonacci(n - 1) + fibonacci(n - 2)


start_time = time.perf_counter_ns()
num = 20
print(fibonacci(num))

end_time = time.perf_counter_ns()
print(f"Execution time: {(end_time - start_time) / 1_000_000:.2f} ms")
print(f"Execution time in us: {(end_time - start_time) / 1_000:.2f} µs")


# 673.04 µs
# 0.67 ms
# for n = 20
