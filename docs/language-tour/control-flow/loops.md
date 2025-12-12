# Loops

## For Loops

```toy
for i in 0..my_int {
    my_float = my_float + 1.0
}
```

For loops can also be used as expressions:

```toy
my_float = for i in 0..5 { my_float + 2.0 }
```

## While Loops

```toy
while my_int < 20 {
    my_int = my_int + 1
}
```

## Loop (infinite)

```toy
loop {
    my_int = my_int + 2
    if my_int >= 30 {
        break
    }
}
```
