# Changelog

## 0.2.0 (unreleased)

- `Frame::stack`: read a stack's statistics while frames are alive
- `Frame::alloc_slice_from_iter`: collect an iterator into a frozen block in place
- `sformat!`: `format!` into a frozen `SStr` on a frame
- `IntoIterator` for `&SSlice`, `&mut SSlice`, `&SVec`, `&mut SVec`, `&SFixedVec`, `&mut SFixedVec`, and by value for `SFixedVec` and `SVec` (`iter::VecIntoIter`, which keeps the frame borrowed so the buffer is reclaimed afterwards)
- Crate docs: *Patterns* now covers phases and choosing a reservation
- `SMap`: a growable, insertion-ordered hash map on a frame (`map` module); grows in place, `swap_remove`/`shift_remove`, iteration in insertion order whatever the hasher
- `SMap::entry` with `or_insert`, `or_insert_with`, `or_default` (`map::Entry`, `OccupiedEntry`, `VacantEntry`)

## 0.1.0 (unreleased)

- `Stack`, `Frame`
- Growable `SVec`, `SString`
- Frozen `SBox`, `SSlice`, `SStr`; fixed-capacity `SFixedVec`
- `AllocError`
- Beyond the design document's list: `SSlice::empty`, `SStr::empty`,
  `SString::capacity`, the `iter` module (to name `IntoIter`), and the
  comparison, hashing and formatting impls on `SFixedVec`
