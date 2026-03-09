fn main() {
    // gives_ownership moves its return
    let s1 = gives_ownership();
                                       // value into s1

    // s2 comes into scope
    let s2 = String::from("hello");

    // s2 is moved into
    let s3 = takes_and_gives_back(s2);
                                       // takes_and_gives_back, which also
                                       // moves its return value into s3
// Here, s3 goes out of scope and is dropped. s2 was moved, so nothing
}
  // happens. s1 goes out of scope and is dropped.

// gives_ownership will move its
fn gives_ownership() -> String {
                                       // return value into the function
                                       // that calls it

    // some_string comes into scope
    let some_string = String::from("yours");

    // some_string is returned and
    some_string
                                       // moves out to the calling
                                       // function
}

// This function takes a String and returns a String.
fn takes_and_gives_back(a_string: String) -> String {
    // a_string comes into
    // scope

    // a_string is returned and moves out to the calling function
    a_string
}
