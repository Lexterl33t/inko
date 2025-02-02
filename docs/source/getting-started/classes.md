---
{
  "title": "Classes"
}
---

Classes are used for storing state used by methods, and instances of classes are
allocated on the heap. One such class we've seen many times so far is the `Main`
class, which defines the main process to run.

Classes are defined using the `class` keyword like so:

```inko
class Person {

}
```

Here `Person` is the name of the class.

## Fields

Fields are defined using the `let` keyword in a `class` body:

```inko
class Person {
  let @name: String
  let @age: Int
}
```

Here we've defined two fields: `name` of type `String`, and `age` of type `Int`.
The `@` symbol isn't part of the name, it's just used to disambiguate the syntax
when referring to fields. Using fields uses the same syntax:

```inko
class Person {
  let @name: String
  let @age: Int

  fn name -> String {
    @name
  }
}
```

Here the `name` method just returns the value of the `@name` field.

The type fields are exposed as depends on the kind of method the field is used
in. If a method is immutable, the field type is `ref T`. If the method is
mutable, the type of a field is instead `mut T`, unless it's defined as a
`ref T`:

```inko
class Person {
  let @name: String
  let @grades: ref Array[Int]
  let @friends: Array[ref Person]

  fn foo {
    @name    # => String
    @grades  # => ref Array[Int]
    @friends # => ref Array[ref Person]
  }

  fn mut foo {
    @name    # => String
    @grades  # => ref Array[Int]
    @friends # => mut Array[ref Person]
  }

  fn move foo {
    @name    # => String
    @grades  # => ref Array[Int]
    @friends # => Array[ref Person]
  }
}
```

If a method takes ownership of its receiver, you can move fields
out of `self`, and the fields are exposed using their original types (i.e.
`@name` is exposed as `String` and not `mut String`).

When moving a field, the remaining fields are dropped individually and the owner
of the moved field is partially dropped. If a type defines a custom destructor,
a `move` method can't move the fields out of its receiver.

## Reopening classes

A class can be reopened using the `impl` keyword like so:

```inko
class Person {
  let @name: String
  let @age: Int
}

impl Person {
  fn greet -> String {
    'Hello ${@name}'
  }
}
```

When reopening a class, only new methods can be added to the class. It's a
compile-time error to try to add a field or overwrite an existing method.

## Swapping field values

Similar to local variables, `:=` can be used to assign a field a new value and
return its old value, instead of dropping the old value:

```inko
class Person {
  let @name: String

  fn mut replace_name(new_name: String) -> String {
    @name := new_name
  }
}
```

## Initialising classes

An instance of a class is created as follows:

```inko
Person(name: 'Alice', age: 42)
```

Here we create a `Person` instance with the `name` field set to `'Alice'`, and
the `age` field set to `42`. We can also use positional arguments, in which case
the order of arguments must match the order in which fields are defined:

```inko
Person('Alice', 42)
```

::: tip
It's recommended to avoid the use of positional arguments when a class defines
more than one field. This ensures that if the order of fields changes, you don't
need to update every line of code that creates an instance of the class.
:::

The fields of an instance can be read from and written to directly, meaning we
don't need to define getter and setter methods:

```inko
let alice = Person(name: 'Alice', age: 42)

alice.name # => 'Alice'
alice.name = 'Bob'
alice.name # => 'Bob'
```

Sometimes creating an instance of a class involves complex logic to assign
values to certain fields. In this case it's best to create a static method to
create the instance for you. For example:

```inko
class Person {
  let @name: String
  let @age: Int

  fn static new(name: String, age: Int) -> Person {
    Person(name: name, age: age)
  }
}
```

Of course nothing complex is happening here, instead we're just trying to
illustrate what using a static method for this might look like.

## Enums

Inko also has "enum classes", created using `class enum`. Enum classes are used
to create sum types, also known as enums:

```inko
class enum Letter {
  case A
  case B
  case C
}
```

Here we've defined a `Letter` enum with three possible cases: `A`, `B`, and `C`.
We can create instances of these cases as follows:

```inko
Letter.A
Letter.B
Letter.C
```

The cases in an enum support arguments, allowing you to store data in them
similar to using regular classes with fields:

```inko
class enum OptionalString {
  case None
  case Some(String)
}
```

We can then create an instance of the `Some` case as follows:

```inko
OptionalString.Some('hello')
```

Unlike other types of classes, you can't use the syntax `OptionalString(...)`
to create an instance of an enum class.

## Value types

While allocating instances of classes on the heap increases flexibility (e.g.
they can be moved around while they're also borrowed), this can reduce
performance when many such instances are allocated.

We can avoid this by using the `inline` modifier when defining a class:

```inko
class inline Number {
  let @value: Int
}
```

The `inline` modifier is also available for enums:

```inko
class inline enum Example {
  case A(Int)
  case B(Float)
}
```

When using this modifier, instances of the class are allocated on the stack and
become _immutable_ value types that are copied upon a move. Unlike their heap
counterparts, such types don't use an object header. For the above `Number`
example that means the memory representation is the same as that of the `Int`
type.

Because these types are immutable, it's not possible to assign fields new values
or define `fn mut` methods on such types. Instead, the approach to "mutation" is
to return a new copy of the instance containing the appropriate changes. For
example:

```inko
class inline Number {
  let @value: Int

  fn increment(amount: Int) -> Number {
    Number(@value + amount)
  }
}
```

Classes defined using the `inline` modifier can only store the following types:

- `Int`, `Float`, `Bool`, `Nil`
- Other `inline` types

Most notably, `String` values can't be stored in an `inline` type since `String`
uses atomic reference counting. This means the following definition is invalid:

```inko
class inline InvalidType {
  let @value: Array[Int] # Array[Int] isn't an `inline` type
}
```

The same restriction applies to generic type parameters:

```inko
class inline Box[T] {
  let @value: T
}

Box([10]) # T requires an `inline` type, but `Array[Int]` isn't such a type
```

It's recommended to use the `inline` modifier whenever possible (i.e. a class
just stores a bunch of `Int` values), provided the above restrictions don't get
in your way of course.

## Processes

Processes are defined using `class async`, and creating instances of such
classes spawns a new process:

```inko
class async Cat {

}
```

Just like regular classes, async classes can define fields using the `let`
keyword:

```inko
class async Cat {
  let @name: String
}
```

Creating instances of such classes is done the same way as with regular classes:

```inko
Cat(name: 'Garfield')
```

Processes can define `async` methods that can be called by other processes:

```inko
class async Cat {
  let @name: String

  fn async give_food {
    # ...
  }
}
```

## Drop order

When dropping an instance of a class with fields, the fields are dropped in
reverse-definition order:

```inko
class Person {
  let @name: String
  let @age: Int
}
```

When dropping an instance of this class, `@age` is dropped before `@name`.

When dropping an `enum` with one or more cases that store data, the data stored
in each case is dropped in reverse-definition order:

```inko
class enum Example {
  case Foo(Int, String)
  case Bar
}
```

When dropping an instance of `Example.Foo`, the `String` value is dropped before
the `Int` value.
