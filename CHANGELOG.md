# Changelog

## 0.2.0 (unreleased)

- `Frame::stack`: read a stack's statistics while frames are alive
- `Frame::alloc_slice_from_iter`: collect an iterator into a frozen block in place
- `sformat!`: `format!` into a frozen `SStr` on a frame

## 0.1.0 (unreleased)

- `Stack`, `Frame`
- Growable `SVec`, `SString`
- Frozen `SBox`, `SSlice`, `SStr`; fixed-capacity `SFixedVec`
- `AllocError`
- Beyond the design document's list: `SSlice::empty`, `SStr::empty`,
  `SString::capacity`, the `iter` module (to name `IntoIter`), and the
  comparison, hashing and formatting impls on `SFixedVec`
