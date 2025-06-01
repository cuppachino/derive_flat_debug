# Derive Flat Debug

This macro generates a custom `Debug` implementation for enums that:

- Flattens single-field tuple variants when the variant name matches the field type name or when `#[debug(flatten)]` is used.
- Skip flattening variants with `#[debug(skip)]`.

Restrictions:

- Only applicable to enums (not structs or unions)
- `#[debug(skip)]` and `#[debug(flatten)]` cannot be used together on the same variant

Example usage:

```rust
#[derive(DebugFlat, PartialEq)]
pub enum QueryToken {
    #[debug(flatten)] Group(Molecule),
    Molecule(Molecule),
    Atom(Atom),
    #[debug(skip)] Infix(Infix),
    #[debug(skip)] Postfix(Postfix),
    Filter(Filter),
}

#[derive(Debug, PartialEq)]
pub struct Molecule(pub Vec<QueryToken>);
```
