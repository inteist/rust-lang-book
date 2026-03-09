fn main() {
    // s comes into scope
    let s = String::from("hello");

    // s's value moves into the function...
    // ... and so is no longer valid here
    takes_ownership(s);

    // x comes into scope
    let x = 5;

    // Because i32 implements the Copy trait,
    // x does NOT move into the function,
    // so it's okay to use x afterward.
    makes_copy(x);

}
// Here, x goes out of scope, then s. However, because s's value was moved,
// nothing special happens.

// some_string comes into scope
fn takes_ownership(some_string: String) {
    println!("{some_string}");
}
// Here, some_string goes out of scope and `drop` is called. The backing
// memory is freed.

// some_integer comes into scope
fn makes_copy(some_integer: i32) {
    println!("{some_integer}");
}
// Here, some_integer goes out of scope. Nothing special happens.
