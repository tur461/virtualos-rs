#[inline]
pub fn slice_to_fixed_array<const N: usize>(slice: &[u8]) -> [u8; N] {
    let mut arr = [0u8; N];
    let len = slice.len().min(N);
    arr[..len].copy_from_slice(&slice[..len]);
    arr
}
