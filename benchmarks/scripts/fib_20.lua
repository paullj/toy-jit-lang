local function fib(n)
  if n == 0 then return 0
  elseif n == 1 then return 1
  else return fib(n-1) + fib(n-2)
  end
end

local start = os.clock()
print(fib(20))
local elapsed = (os.clock() - start) * 1000000
print(string.format("%.2f", elapsed))
