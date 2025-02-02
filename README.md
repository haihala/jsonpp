# JSON++

aka jsonpp, jsonxx, or whatever you want to call it.

A language somewhere between MS Excel and json with comments and trailing
commas. The key ability is to calculate a value based on other values in the
file.

A valid json file or even a json file with comments should be a valid JSON++
file, but it won't go the other way. Not all JSON++ files are valid json.

## The language

It is:

- A joke
- Garbage
- Purely functional

These qualities are independent of each other. Be not afraid of the red in the
syntax highlighting, GitHub simply doesn't comprehend the awesomeness of JSON++.
Yet.

To make a value interactive, you can call functions with our lisp-like syntax:

```json
{
    "key1": "1",
    "key2": (sum 2, 1),
}
```

Will evaluate to:

```json
{
  "key1": "1",
  "key2": 3
}
```

There are cases when some function demands a specific type. JSON++ has mostly
the same types as regular JSON, those being:

- number is split to int and float while evaluating and then recombined to number in the output
- string, double quoted
- array
- object
- bool
- null

### Functions

There are a bunch of useful functions in the language. Which unlike in Excel,
won't get translated because I'm not that committed to the joke. Some of these
include:

- `(sum a, b, c, d...)` - Calculates the sum of all the elements
- `(sub a, b)` - a-b
- `(mul a, b, c, d...)` - Calculates the product of all the elements
- `(div a, b)` - a/b, will exit if b is zero
- `(pow a, b)` - Raises a to the power of b
- `(log a, b)` - a based Logarithm of b, `log(2, 8)` would output 3
- `(mod a, b)` - Remainder when dividing a by b
- `(max a, b)` - Returns the greater of two numeric values
- `(min a, b)` - Returns the lesser of two numeric values
- `(len a)` - Returns the length of a (string, object, array)
- `(str a)` - Returns a as a string
- `(int a)` - Attempts to parse an integer out of a
- `(float a)` - Attempts to parse a float out of a
- `(concat a, b)` - Concatenates strings and arrays

#### Ref

`(ref path)` will evaluate to the value in the cell in the given path. Here are
some valid paths:

- `(ref foo)` - Value on the root object with the key 'foo'
- `(ref .foo)` - Sibling value with the key 'foo' on the same object
- `(ref foo.bar)` - Value of key 'bar' within the value of the key 'foo' within object root
- `(ref .foo.bar)` - Value of key 'bar' within the value of the key 'foo', which is a sibling of current cell
- `(ref .)` - Parent of current cell
- `(ref ..)` - Parent of the parent of current cell
- `(ref ...)` - Parent of the parent of the parent of current cell, this keeps going
- `(ref [1])` - Assumes root node is an array, references the second element
- `(ref .[1])` - Second sibling element (referer is another element in the same list)
- `(ref .[-1])` - Previous sibling element, maybe later also next child
- `(ref foo.[1])` - Second child of the array that is under key 'foo' under root
- `(ref foo.[_])` - Every child of the array that is under key 'foo' under root
- `(ref foo.[_].name)` - Name of every child of the array that is under key 'foo' under root

Ref is probably the most important function. It allows you to reference a
different 'cell'. When the initial file is parsed, dependencies between cells
are taken into account. If at any point during the evaluation process, a cyclic
dependency is detected, the program exits with an error message.

#### Import and include

`(include path)` will work similar to include in languages like c. It will look
for a file in the file system path and return its contents into this cell as a
string.

`(import path)` works like include, except it assumes the contents of that file
are more JSON++, so it parses that file. Somewhat similar output to `ref`.

#### Conditionals

The `(if cond, a, b)` value works similar to excel. It evaluates to a if cond is
truthy and b is cond is falsy. Truthy values include:

- true
- Non-empty strings, objects and arrays
- Non-zero numbers

Falsy values are:

- 0
- false
- null
- ""
- {}
- []

You also have access to basic comparison features such as ==, >, <, >=, and <=.

#### Fold

To iterate and aggregate, you can use `fold`. It takes two arguments, a function
and a collection. The function should be one that takes two arguments and
returns whatever. It'll then push the entire collection through that function.
This is enough to implement map, filter and reduce like so:

```
// Reduce, take a list, evaluate to a single value
(fold (ref some.list), (acc, elem) => (f acc, elem))

// Map
(fold (ref some.list), (acc, elem) => (concat [acc], [(f elem)]))

// Filter
(fold((ref some.list), (acc, elem) => (if (f elem), (concat [acc], [elem]), [acc]))
```

All of these have been provided out of convenience, where you simply input the
collection followed by the function denoted as f in the examples above.

### Data structures

#### Ranges

`a..b` will produce a range from a to b (non-inclusive). This only works if a
and b are integers. These work similarly to arrays of integers

#### Arrays

Arrays can be indexed with the angle brackets `[n]`. Zero is the first element
and positive indices count from the beginning. Negative indices count from the
back. Indexing with a range produces a slice that corresponds to that range.

#### Objects

Object values can be accessed with angle brackets similar to arrays, except you
have to use a string as a key.

## IO

Input can be read from a file with the `--input` cli argument, or from stdin if
the argument is omited. Output will be sent to stdout unless `--output` is
provided, in which case it'll be saved to that file.
