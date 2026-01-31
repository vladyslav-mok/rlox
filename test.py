#!/usr/bin/env python3
# Performance test ported from Lox to Python 3

import time

# ========== HELPER FUNCTIONS ==========
def bench_start(name):
    print("=== " + name + " ===")
    start = time.time()
    return start

def bench_end(start_time, name, count):
    end = time.time()
    duration = end - start_time
    print(f"Operations: {count}")
    print(f"Time: {duration} sec")
    if duration > 0:
        ops_per_sec = count / duration
        print(f"Ops/sec: {ops_per_sec}")
    print()

# ========== TEST: VARIABLES ==========
def test_variables():
    start = bench_start("Variables")
    count = 0
    for i in range(10000):
        a = i
        b = i + 1
        c = i + 2
        d = a + b + c
        count = count + 1
    bench_end(start, "Variables", count)

# ========== TEST: ARITHMETIC ==========
def test_arithmetic():
    start = bench_start("Arithmetic Operations")
    count = 0
    for i in range(10000):
        a = i + 5
        b = i - 3
        c = i * 2
        d = i / 2
        e = i % 3
        count = count + 5
    bench_end(start, "Arithmetic Operations", count)

# ========== TEST: COMPARISON ==========
def test_comparison():
    start = bench_start("Comparison Operations")
    count = 0
    for i in range(10000):
        a = i == 5000
        b = i != 5000
        c = i < 5000
        d = i > 5000
        e = i <= 5000
        f = i >= 5000
        count = count + 6
    bench_end(start, "Comparison Operations", count)

# ========== TEST: LOGICAL ==========
def test_logical():
    start = bench_start("Logical Operations")
    count = 0
    for i in range(10000):
        a = not True
        b = i < 5000 and i > 0
        c = i < 0 or i > 5000
        d = True and False
        e = True or False
        count = count + 5
    bench_end(start, "Logical Operations", count)

# ========== TEST: STRINGS ==========
def test_strings():
    start = bench_start("String Operations")
    count = 0
    result = ""
    for i in range(1000):
        result = result + "a"
        count = count + 1
    bench_end(start, "String Operations", count)

# ========== TEST: CONDITIONALS ==========
def test_conditionals():
    start = bench_start("Conditional Statements")
    count = 0
    for i in range(10000):
        if i < 5000:
            count = count + 1
        else:
            count = count + 1
    bench_end(start, "Conditional Statements", count)

# ========== TEST: NESTED CONDITIONS ==========
def test_nested_conditions():
    start = bench_start("Nested Conditions")
    count = 0
    for i in range(10000):
        if i < 3333:
            if i < 1666:
                count = count + 1
            else:
                count = count + 1
        elif i < 6666:
            if i < 5000:
                count = count + 1
            else:
                count = count + 1
        else:
            count = count + 1
    bench_end(start, "Nested Conditions", count)

# ========== TEST: WHILE LOOP ==========
def test_while_loop():
    start = bench_start("While Loop")
    count = 0
    i = 0
    while i < 10000:
        i = i + 1
        count = count + 1
    bench_end(start, "While Loop", count)

# ========== TEST: FOR LOOP ==========
def test_for_loop():
    start = bench_start("For Loop")
    count = 0
    for i in range(10000):
        count = count + 1
    bench_end(start, "For Loop", count)

# ========== TEST: NESTED LOOPS ==========
def test_nested_loops():
    start = bench_start("Nested Loops")
    count = 0
    for i in range(100):
        for j in range(100):
            count = count + 1
    bench_end(start, "Nested Loops", count)

# ========== TEST: FUNCTIONS ==========
def test_functions():
    start = bench_start("Function Calls")
    def simple_func(x):
        return x + 1
    count = 0
    for i in range(10000):
        result = simple_func(i)
        count = count + 1
    bench_end(start, "Function Calls", count)

# ========== TEST: MULTIPLE PARAMETERS ==========
def test_multiple_params():
    start = bench_start("Functions with Multiple Parameters")
    def add(a, b, c):
        return a + b + c
    count = 0
    for i in range(10000):
        result = add(i, i + 1, i + 2)
        count = count + 1
    bench_end(start, "Functions with Multiple Parameters", count)

# ========== TEST: RECURSION ==========
def test_recursion():
    start = bench_start("Recursion (fib(25))")
    def fib(n):
        if n <= 1:
            return n
        return fib(n - 1) + fib(n - 2)
    fib_result = fib(25)
    count = 196418
    bench_end(start, "Recursion (fib(25))", count)

# ========== TEST: CLOSURES ==========
def test_closures():
    start = bench_start("Closures")
    def make_adder(x):
        def adder(y):
            return x + y
        return adder
    count = 0
    add10 = make_adder(10)
    for i in range(10000):
        result = add10(i)
        count = count + 1
    bench_end(start, "Closures", count)

# ========== TEST: CLASS CREATION ==========
def test_class_creation():
    start = bench_start("Class Object Creation and Usage")
    class SimpleClass:
        def __init__(self, value):
            self.value = value

        def get_value(self):
            return self.value

        def set_value(self, value):
            self.value = value
    count = 0
    obj = SimpleClass(0)
    for i in range(10000):
        obj.set_value(i)
        v = obj.get_value()
        count = count + 2
    bench_end(start, "Class Object Creation and Usage", count)

# ========== TEST: MULTIPLE OBJECTS ==========
def test_multiple_objects():
    start = bench_start("Multiple Object Creation")
    class TestClass:
        def __init__(self, a, b):
            self.a = a
            self.b = b

        def compute(self):
            return self.a + self.b
    count = 0
    for i in range(1000):
        obj = TestClass(i, i + 1)
        result = obj.compute()
        count = count + 1
    bench_end(start, "Multiple Object Creation", count)

# ========== TEST: INHERITANCE ==========
def test_inheritance():
    start = bench_start("Inheritance and Virtual Methods")
    class Parent:
        def __init__(self, name):
            self.name = name

        def speak(self):
            return self.name + " speaks"

    class Child(Parent):
        def __init__(self, name):
            super().__init__(name)

        def speak(self):
            return self.name + " speaks loudly"
    count = 0
    child = Child("Child")
    for i in range(10000):
        result = child.speak()
        count = count + 1
    bench_end(start, "Inheritance and Virtual Methods", count)

# ========== TEST: SCOPE ==========
def test_scope():
    start = bench_start("Scope and Blocks")
    count = 0
    for i in range(1000):
        outer = i
        if True:
            inner = i + 1
            count = count + 1
        if True:
            inner2 = i + 2
            count = count + 1
        count = count + 1
    bench_end(start, "Scope and Blocks", count)

# ========== TEST: COMPLEX EXPRESSIONS ==========
def test_complex_expressions():
    start = bench_start("Complex Expressions")
    count = 0
    for i in range(10000):
        result = ((i + 5) * 2 - 3) / 4 + ((i * 3) + 1) / 2
        count = count + 1
    bench_end(start, "Complex Expressions", count)

# ========== TEST: ASSIGNMENT ==========
def test_assignment():
    start = bench_start("Assignment")
    count = 0
    a = 0
    b = 0
    c = 0
    d = 0
    e = 0
    for i in range(10000):
        a = i
        b = i + 1
        c = i + 2
        d = i + 3
        e = i + 4
        count = count + 5
    bench_end(start, "Assignment", count)

# ========== TEST: NIL HANDLING ==========
def test_nil_handling():
    start = bench_start("Nil Handling")
    count = 0
    for i in range(10000):
        x = None
        if x is None:
            x = i
        if x is not None:
            count = count + 1
    bench_end(start, "Nil Handling", count)

# ========== TEST: LINKED LIST ==========
def test_linked_list():
    start = bench_start("Linked List (Data Structures)")
    class Node:
        def __init__(self, value):
            self.value = value
            self.next = None

        def add_next(self, node):
            self.next = node

        def traverse(self):
            sum_val = 0
            current = self
            count = 0
            while current is not None:
                sum_val = sum_val + current.value
                current = current.next
                count = count + 1
            return count
    count = 0
    head = Node(0)
    current = head
    for i in range(1, 1000):
        new_node = Node(i)
        current.add_next(new_node)
        current = new_node
        count = count + 1
    traversed = head.traverse()
    count = count + traversed
    bench_end(start, "Linked List (1000 elements)", count)

# ========== TEST: CLASS METHODS ==========
def test_class_methods():
    start = bench_start("Class Methods")
    class Calculator:
        def __init__(self):
            self.value = 0

        def add(self, n):
            self.value = self.value + n

        def mul(self, n):
            self.value = self.value * n

        def get(self):
            return self.value
    count = 0
    calc = Calculator()
    for i in range(10000):
        calc.add(1)
        calc.mul(2)
        v = calc.get()
        count = count + 3
    bench_end(start, "Class Methods", count)

# ========== TEST: NESTED CLASSES ==========
def test_nested_classes():
    start = bench_start("Complex Class Operations")
    class Counter:
        def __init__(self, start):
            self.value = start

        def increment(self):
            self.value = self.value + 1

        def decrement(self):
            self.value = self.value - 1

        def get_value(self):
            return self.value

    class Controller:
        def __init__(self):
            self.counter = Counter(0)

        def update(self):
            self.counter.increment()
            return self.counter.get_value()
    count = 0
    controller = Controller()
    for i in range(10000):
        v = controller.update()
        count = count + 1
    bench_end(start, "Complex Class Operations", count)

# ========== TEST: MULTIPLE VARIABLES ==========
def test_multiple_variables():
    start = bench_start("Multiple Variable Changes")
    count = 0
    x = 0
    y = 1
    z = 2
    for i in range(10000):
        x = x + y
        y = y + z
        z = z + x
        x = x - 1
        y = y - 1
        z = z - 1
        count = count + 6
    bench_end(start, "Multiple Variable Changes", count)

# ========== TEST: COMBINED ==========
def test_combined():
    start = bench_start("Combined Test")
    def compute(x):
        if x < 0:
            return 0
        elif x > 100:
            return 100
        return x * 2

    class Processor:
        def __init__(self):
            self.total = 0

        def process(self, value):
            self.total = self.total + compute(value)

        def get_total(self):
            return self.total
    count = 0
    processor = Processor()
    for i in range(10000):
        processor.process(compute(i))
        count = count + 1
    total = processor.get_total()
    count = count + 1
    bench_end(start, "Combined Test", count)

# ========== RUN ALL TESTS ==========
if __name__ == "__main__":
    test_variables()
    test_arithmetic()
    test_comparison()
    test_logical()
    test_strings()
    test_conditionals()
    test_nested_conditions()
    test_while_loop()
    test_for_loop()
    test_nested_loops()
    test_functions()
    test_multiple_params()
    test_recursion()
    test_closures()
    test_class_creation()
    test_multiple_objects()
    test_inheritance()
    test_scope()
    test_complex_expressions()
    test_assignment()
    test_nil_handling()
    test_linked_list()
    test_class_methods()
    test_nested_classes()
    test_multiple_variables()
    test_combined()

    print("========================================")
    print("=== All Performance Tests Completed! ===")
    print("========================================")
    print()
    print("Summary:")
    print("The time.time() function is used to measure time")
    print("in seconds between start and end of each test.")
