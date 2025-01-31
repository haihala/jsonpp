# JSON++

aka jsonpp, jsonxx, or whatever you want to call it.

A language somewhere between MS Excel and json with comments and trailing
commas. The key ability is to calculate a value based on other values in the
file.

A valid json file or even a json file with comments should be a valid json++
file, but it won't go the other way. Not all json++ files are valid json.

## The language

It is:

- A joke
- Garbage
- Purely functional

These qualities are independent of each other.

To make a value interactive, have it start with an equals sign (=) similar to
Excel. For example:

```json
{
    "key1": "1",
    "key2": =2,
}
```

Will evaluate to:

```json
{
  "key1": "1",
  "key2": 2
}
```

You can do basic math with familiar symbols, all of these are valid:

```json
[
    =1+1,
    =5-3,
    =2/3,   // Dividing by zero will exit the program with an error message
    =4*5,
]
```

### Functions

There are a bunch of useful functions in the language. Which unlike in Excel,
won't get translated because I'm not that committed to the joke. Some of these
include:

- `pow(a, b)` - Raises a to the power of b
- `log(a, b)` - a based Logarithm of b, `log(2, 8)` would output 3
- `mod(a, b)` - Remainder when dividing a by b

#### Ref

`ref(path)` will evaluate to the value in the cell in the given path. Here are
some valid paths:

- `ref(foo)` - Value on the root object with the key 'foo'
- `ref(.foo)` - Sibling value with the key 'foo' on the same object
- `ref(foo.bar)` - Value of key 'bar' within the value of the key 'foo' within object root
- `ref(.foo.bar)` - Value of key 'bar' within the value of the key 'foo', which is a sibling of current cell
- `ref(.)` - Parent of current cell
- `ref(..)` - Parent of the parent of current cell
- `ref(...)` - Parent of the parent of the parent of current cell, this keeps going
- `ref([1])` - Assumes root node is an array, references the second element
- `ref(.[1])` - Second sibling element (referer is another element in the same list)
- `ref(.[-1])` - Previous sibling element, maybe later also next child
- `ref(foo.[1])` - Second child of the array that is under key 'foo' under root
- `ref(foo.[_])` - Every child of the array that is under key 'foo' under root
- `ref(foo.[_].name)` - Name of every child of the array that is under key 'foo' under root

Ref is probably the most important function. It allows you to reference a
different 'cell'. When the initial file is parsed, dependencies between cells
are taken into account. If at any point during the evaluation process, a cyclic
dependency is detected, the program exits with an error message.

#### Import and include

`include(path)` will work similar to include in languages like c. It will look
for a file in the file system path and return its contents into this cell as a
string.

`import(path)` works like include, except it assumes the contents of that file
are more json++, so it parses that file. Somewhat similar output to `ref`.

## IO

Input can be read from a file with the `--input` cli argument, or from stdin if
the argument is omited. Output will be sent to stdout unless `--output` is
provided, in which case it'll be saved to that file.
