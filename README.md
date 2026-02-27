# Memory Box

Small, pragmatic Rust library for **local process memory manipulation** — mainly useful when developing **DLL mods**, **game trainers**, or memory-based hooks on Windows.

**Status:** Experimental / Alpha — API is still evolving.
**Platform:** Windows (can work at linux if your game or application are running through proton, wine, bottles, etc)

---

## Installation

```toml
memory_box = { git = "https://github.com/zaarov/MemoryBox.git" }
```

---

## Key concepts & types

### `MemoryError`

An enum describing low-level failure modes: `NullPointer`, `InvalidLength`, `VirtualProtectFailed`, `ReadFailed`, `WriteFailed`, `OutOfBounds`. All public APIs return `Result<..., MemoryError>`.

### `ModuleContext`

Represents the current module (the process / DLL where the code runs):

* `ModuleContext::current() -> Result<ModuleContext, MemoryError>` — queries Windows (`GetModuleHandleW` + `GetModuleInformation`) to obtain module base + size.

* `pattern_scan(&self, pattern: &[Option<u8>]) -> Result<LocalPtr, MemoryError>` — reads the module via `ReadProcessMemory` into a `Vec<u8>` and searches for the pattern with wildcards.

### `LocalPtr`

A small wrapper around a raw address in the current process (`usize`). Methods return `Result` and are crash-safe:

* `from_addr(address: usize) -> LocalPtr` — construct from address.

* `offset(&self, off: isize) -> Result<LocalPtr, MemoryError>` — add/subtract offset with checked arithmetic.

* `read_bytes(&self, len: usize) -> Result<Vec<u8>, MemoryError>` — read bytes using `ReadProcessMemory`. Returns `Err` instead of causing an access violation when memory is unreadable.

* `deref(&self) -> Result<LocalPtr, MemoryError>` — read a pointer-sized value using `read_bytes` (handles both 32- and 64-bit).

* `rip_relative(&self, offset_offset: isize, instruction_len: isize) -> Result<LocalPtr, MemoryError>` — resolve RIP-relative addressing (reads a 32-bit displacement and computes target).

* `write_bytes(&self, data: &[u8]) -> Result<(), MemoryError>` — direct local write (kept for internal uses).

* `write_bytes_protected(&self, data: &[u8]) -> Result<(), MemoryError>` — change page protections and write safely via `WriteProcessMemory`, then `FlushInstructionCache`.

* `chain(self) -> LocalPtrChain` — start a fluent pointer-chasing chain.

### `LocalPtrChain`

Fluent-style API for pointer chasing; methods return `Result`:

* `offset(self, off: isize) -> Result<Self, MemoryError>`

* `deref(self) -> Result<Self, MemoryError>`

* `finish(self) -> LocalPtr`

---

## Pattern format
Patterns are `&[Option<u8>]`. Example:

* `Some(0x48)` means the byte must match 0x48.
 
* `None` is a wildcard (`??`).

Example pattern used in the library:
```rust
// pattern: Some(0xDE), Some(0xAD), None, Some(0xBE) means "DE AD ?? BE"
const GAME_MANAGER_IMP: [Option<u8>; 17] = [
    Some(0x48),
    Some(0x8B), 
    Some(0x05),
    None, 
    None, 
    None, 
    None,
    Some(0x48), 
    Some(0x8B), 
    Some(0x58), 
    Some(0x38),
    Some(0x48), 
    Some(0x85), 
    Some(0xDB), 
    Some(0x74),
    None, 
    Some(0xF6),
];
```

### Examples

## Resolve a pointer chain and read an `f32` (safe)

```rust
use memory_box::{ModuleContext, MemoryError};

fn read_health() -> Result<f32, MemoryError> {
    let ptr = ModuleContext::current()?
        .pattern_scan(&GAME_MANAGER_IMP)?
        .rip_relative(3, 7)?
        .deref()?
        .chain()
        .offset(0x18)?
        .deref()?
        .offset(0x310)?
        .deref()?
        .offset(0xD8)?
        .deref()?
        .offset(0x1C8)?
        .offset(0x60C)?
        .finish();

    let bytes = ptr.read_bytes(4)?;
    let value = f32::from_le_bytes(bytes.try_into().map_err(|_| MemoryError::ReadFailed)?);
    Ok(value)
}
```

## Read a primitive helper
```rust
fn read_addr_as_i32(addr: LocalPtr) -> Result<i32, MemoryError> {
    let bytes = addr.read_bytes(4)?;
    Ok(i32::from_le_bytes(bytes.try_into().map_err(|_| MemoryError::ReadFailed)?))
}
```
## Protected write (patch code/data)

```rust
let target: LocalPtr = LocalPtr::from_addr(0x7FF6_1234_0000);
let new_bytes: [u8; 5] = [0x90, 0x90, 0x90, 0x90, 0x90];
target.write_bytes_protected(&new_bytes)?;
```
`write_bytes_protected` will:

* call `VirtualProtect` to set `PAGE_READWRITE`,

* write with `WriteProcessMemory`,

* restore the old protection,

* call `FlushInstructionCache`.