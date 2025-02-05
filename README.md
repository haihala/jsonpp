# JSON++

jsonpp, json++, jsonxx, or whatever you want to call it is a JSON PreProcessor.

A language somewhere between MS Excel and json with comments and trailing
commas. The key ability is to calculate a value based on other values in the
file.

A valid json file or even a json file with comments should be a valid jsonpp
file, but it won't go the other way. Not all jsonpp files are valid json.

## The language

It is:

- A garbage joke
- Functional
- Interpreted

These qualities are independent of each other. Be not afraid of the red in the
syntax highlighting, GitHub simply doesn't comprehend the awesomeness of jsonpp.
Yet. Most of what is on here works, but there are still lots of known bugs.
Please for the love of everything you hold dear, don't use this for anything.

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

There are cases when some function demands a specific type. jsonpp has mostly
the same types as regular JSON, those being:

- number is split to int and float while evaluating and then recombined to number in the output
- string, double quoted
- array
- object
- bool
- null
- undefined, will get stripped out of the final output

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
- `(merge a, b)` - Concatenates strings and arrays, combines objects

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
are more jsonpp, so it parses that file. Somewhat similar output to `ref`.

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
- undefined
- ""
- {}
- []

You also have access to basic comparison functions such as `eq`, `lt`, `gt`,
`lte`, and `gte`

#### Folds

The language offers plenty of tools for manipulating data structures. Since all
of them could be implemented with a fold, they are called folds. There are three
available folds as of writing, `map`, `filter`, and `reduce`. See example:

```json
{
  "map": (map (def x, (mul 2, x)), (range 1, 10)),
  "filter": (filter (def x, (eq 0, (mod x, 3))), (range 1, 10)),
  "reduce": (reduce sum, (range 1, 10)),
}
```

evaluates to

```json
{
  "filter": [3, 6, 9],
  "map": [2, 4, 6, 8, 10, 12, 14, 16, 18],
  "reduce": 45
}
```

### Data structures

Arrays and Objects. Like JSON. Arrays of integers can be generated with the
`(range start end)` function.

## IO

Input can be read from a file with the `--input` cli argument, or from stdin if
the argument is omited. Output will be sent to stdout unless `--output` is
provided, in which case it'll be saved to that file.
