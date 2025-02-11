# JSON++

JSON PreProcessor. Written in rust, so it's blazingly fast 🚀.

It's a language somewhere between MS Excel and json with comments and trailing
commas. The key ability is to calculate values dynamically based on other values
in the file.

A valid json file or even a json file with comments is a valid jsonpp file, but
all jsonpp files are not valid json.

## The language

It is:

- Functional
- Interpreted
- A garbage joke

These qualities are independent of each other. Be not afraid of the red in the
syntax highlighting, GitHub simply doesn't comprehend the awesomeness of jsonpp.
Yet. Most of what is on here works, but there are still lots of known bugs.
Please for the love of everything you hold dear, don't use this for anything.
The error messages especially will be incredibly horrendous.

To make a value interactive, you can call functions with our lisp-like syntax:

```json
{
    "key1": "1",
    "key2": (sum 2 1),
}
```

Will evaluate to:

```json
{
  "key1": "1",
  "key2": 3
}
```

Some function demands a specific type. jsonpp has a simple set of types:

- int for integers
  - Will be output as a number
- float for floating point numbers
  - Will be output as a number
- string
  - Double quoted
- array, heterogeneous
- object
- bool
- null
- undefined
  - Will get stripped out of the final output
  - Usable for conditional fields
- definition
  - JsonPP internal
  - Will get stripped out of the final output
- identifier
  - JsonPP internal
  - Dangling identifiers will cause an error
- dynamic, aka function call
  - JsonPP internal
  - All of these must be evaluated

### Functions

There are a bunch of useful functions in the language. Which unlike in Excel,
won't get translated because I'm not that committed to the joke. Some of these
include:

- `(sum a b c d...)` - Calculates the sum of all the elements
- `(sub a b)` - a-b
- `(mul a b c d...)` - Calculates the product of all the elements
- `(div a b)` - a/b, will exit if b is zero
- `(pow a b)` - Raises a to the power of b
- `(log a b)` - a based Logarithm of b, `(log 2 8)` would output 3
  - No base 1
- `(mod a b)` - Remainder when dividing a by b
- `(max a b)` - Returns the greater of two numeric values
  - Comparing int and a float will output a float, value may be from the int
- `(min a b)` - Returns the lesser of two numeric values
  - Comparing int and a float will output a float, value may be from the int
- `(len a)` - Returns the length of a (string, object, array)
- `(str a)` - Returns a as a string
- `(int a)` - Attempts to parse an integer out of a
  - Will round the input if it has decimal places, "0.5" -> 1
- `(float a)` - Attempts to parse a float out of a
- `(merge a b)` - Concatenates strings and arrays, combines objects

#### Ref

Ref is the most important function. It allows you to reference a different
'cell'. `(ref path)` will evaluate to the value in the given path. It can accept
relative paths (start with a period) or absolute paths (everything else). The
path must be a string, but can be dynamically generated.

You have four path chunks that can be chained together in any order:

- `.`, "parent", refers to the parent of the element
- `[foo]`, "array index", allows indexing into arrays
- `(foo)`, "parameter index", allows indexing into dynamic cell parameters
  - Parameter 0 is the callable, after that you have args in order
- `foo`, "object key", allows selecting a key of an object

Here are some valid paths and what they point to:

- `foo` - Key 'foo' under the object root
- `foo.bar` - Key bar of the value of key 'foo' under the object root
- `foo.[2]` - Third element of the array that is the value of key 'foo' under the object root
- `foo.(2)` - Second parameter of the dynamic that is the value of key 'foo' under the object root
- `.` - The ref call itself
- `..` - Parent of the ref call
- `...` - Grandparent of the ref call

When using a relative path, you should think of the path relative to the ref
call. `(ref ".")` is a ref call that points to itself. This is useful, since ref
accepts any number of arguments, meaning you can use it to access values in
itself like this: `(ref ".(2).name" {"name": "foo"})` will resolve to` "foo"`

Absolute paths are absolute relative to the primary jsonpp root.

#### Import and include

`(include path)` will work similar to include in languages like c. It will look
for a file in the file system path and return its contents into this cell as a
string.

`(import path)` works like include, except it assumes the file contains jsonpp
and parses that file. This has an effect when importing something that uses
`ref` to reference something absolutely. Absolute refs are relative to the
primary file, meaning if you run file A and it imports file B in some sub-field,
absolute refs in file B will be relative to the root of file A.

Paths for both are relative to the working directory of the shell that invoked

`jsonpp`.

#### Conditionals

The `(if cond a b)` value works similar to excel. It evaluates to a if cond is
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
`lte`, and `gte`. To invert something use `not`.

#### Folds

The language offers plenty of tools for manipulating data structures. Since all
of them could be implemented with a fold, they are called folds. There are three
available folds as of writing, `map`, `filter`, and `reduce`. See example:

```json
{
  "map": (map (def x (mul 2 x)) (range 1 10)),
  "filter": (filter (def x (eq 0 (mod x 3))) (range 1 10)),
  "reduce": (reduce sum (range 1 10)),
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

As of writing, object keys must be hard-coded strings. I'm open to a PR if some
psycho puts in the few hours to make it happen.

## IO

You cannot read input in jsonpp (yet), but the interpreter accepts input via
file or stdin. Input can be read from a file with the `--input` cli argument, or
from stdin if the argument is omitted. Output will be sent to stdout unless
`--output` is provided, in which case it'll be saved to that file. If the file
already exists, you need to use `--force` to override it.
