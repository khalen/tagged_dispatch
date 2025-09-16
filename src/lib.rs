//! # tagged_dispatch
//!
//! Memory-efficient trait dispatch using tagged pointers. Like `enum_dispatch`, but uses
//! only 8 bytes per instance with heap-allocated variants instead of stack-allocated ones
//! the size of the largest variant.
//!
//! ## When to Use
//!
//! Choose `tagged_dispatch` when:
//! - You have many instances and memory usage is critical (8 bytes vs potentially hundreds)
//! - Your variants are large or vary significantly in size
//! - You can accept the heap allocation overhead
//!
//! Choose `enum_dispatch` when:
//! - You want stack allocation and no heap overhead
//! - Your variants are similarly sized or small
//! - You have fewer instances
//!
//! Choose trait objects when:
//! - You need open sets of types (not known at compile time)
//! - You're okay with 16-byte fat pointers
//!
//! ## Architecture Requirements
//!
//! This crate requires x86-64 or AArch64 architectures where the top 7 bits of
//! 64-bit pointers are unused (standard on modern Linux, macOS, and Windows systems).
//!
//! ## Limitations
//!
//! - Supports up to 128 variant types (7-bit tag)
//! - Generic traits are not yet supported
//! - Requires heap allocation for variants
//!
//! ## Memory Models
//!
//! ### Owned Mode (default)
//!
//! Without lifetime parameters, generates owned tagged pointers using `Box`:
//! - Variants are allocated with `Box::into_raw(Box::new(value))`
//! - Implements `Drop` to deallocate
//! - Has non-trivial `Clone` that deep-copies
//!
//! ### Arena Mode
//!
//! With lifetime parameters, generates arena-allocated pointers:
//! - Variants allocated through `TaggedAllocator` trait
//! - Types are `Copy` (just copies the 8-byte pointer)
//! - Arena manages object lifetimes
//! - Variants don't need to be `Send`, `Sync`, or even `Sized`
//!
//! ## Features
//!
//! - `std` (default): Standard library support
//! - `allocator-bumpalo`: Implements `TaggedAllocator` for `bumpalo::Bump`
//! - `allocator-typed-arena`: Implements `TaggedAllocator` for `typed_arena::Arena<T>`
//! - `all-allocators`: Enables all allocator implementations
//!
//! ## Example
//!
//! ```rust
//! use tagged_dispatch::tagged_dispatch;
//!
//! #[tagged_dispatch]
//! trait Draw {
//!     fn draw(&self);
//! }
//!
//! #[tagged_dispatch(Draw)]
//! enum Shape {
//!     Circle,      // Automatically expands to Circle(Circle)
//!     Rectangle,   // Expands to Rectangle(Rectangle)
//! }
//!
//! struct Circle { radius: f32 }
//! impl Draw for Circle {
//!     fn draw(&self) { println!("Drawing circle"); }
//! }
//!
//! struct Rectangle { width: f32, height: f32 }
//! impl Draw for Rectangle {
//!     fn draw(&self) { println!("Drawing rectangle"); }
//! }
//!
//! // Use it - only 8 bytes!
//! let shape = Shape::circle(Circle { radius: 1.0 });
//! shape.draw();
//! assert_eq!(std::mem::size_of::<Shape>(), 8);
//! ```
//!
//! ## Arena Allocation
//!
//! When you add a lifetime parameter, the macro generates an arena builder pattern
//! that supports both bumpalo and typed-arena:
//!
//! ```rust
//! use tagged_dispatch::tagged_dispatch;
//!
//! #[tagged_dispatch(Draw)]
//! enum ArenaShape<'a> { // Triggers generation of arena builder
//!     Circle,
//!     Rectangle,
//! }
//!
//! // Create a builder (automatically chooses best allocator)
//! let builder = ArenaShape::arena_builder();
//! let shape = builder.circle(Circle { radius: 1.0 });
//! let shape2 = shape;  // Copy - just 8 bytes!
//!
//! // Explicitly use bumpalo (when feature enabled)
//! #[cfg(feature = "allocator-bumpalo")]
//! {
//!     let builder = ArenaShapeArenaBuilder::with_bumpalo();
//!     let shape = builder.circle(Circle { radius: 1.0 });
//! }
//!
//! // Explicitly use typed-arena (when feature enabled)
//! #[cfg(feature = "allocator-typed-arena")]
//! {
//!     let builder = ArenaShapeArenaBuilder::with_typed_arena();
//!     let shape = builder.circle(Circle { radius: 1.0 });
//! }
//!
//! // Reset allocations for batch processing
//! let mut builder = ArenaShape::arena_builder();
//! builder.reset(); // Invalidates all previous allocations
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

use core::marker::PhantomData;

#[cfg(not(feature = "std"))]
use alloc::boxed::Box;
#[cfg(feature = "std")]
use std::boxed::Box;

// Re-export the macro
pub use tagged_dispatch_macros::tagged_dispatch;

/// The core tagged pointer type used internally.
/// 
/// Uses the top 7 bits of a 64-bit pointer for type tagging,
/// supporting up to 128 different types while maintaining an 8-byte size.
#[repr(transparent)]
pub struct TaggedPtr<T> {
    ptr: usize,
    _phantom: PhantomData<T>,
}

impl<T> TaggedPtr<T> {
    const TAG_SHIFT: usize = 57;
    const TAG_MASK: usize = 0x7F << Self::TAG_SHIFT;
    const PTR_MASK: usize = !Self::TAG_MASK;
    
    /// Maximum number of variants supported (2^7 = 128)
    pub const MAX_VARIANTS: usize = 128;
    
    /// Create a new tagged pointer
    #[inline(always)]
    pub fn new(ptr: *mut T, tag: u8) -> Self {
        debug_assert!(
            tag < Self::MAX_VARIANTS as u8,
            "Tag must be less than 128 (7 bits)"
        );
        
        let addr = ptr as usize;
        debug_assert_eq!(
            addr & Self::TAG_MASK, 
            0, 
            "Pointer already has high bits set!"
        );
        
        Self {
            ptr: addr | ((tag as usize) << Self::TAG_SHIFT),
            _phantom: PhantomData,
        }
    }
    
    /// Get the tag value
    #[inline(always)]
    pub fn tag(&self) -> u8 {
        ((self.ptr & Self::TAG_MASK) >> Self::TAG_SHIFT) as u8
    }
    
    /// Get the untagged pointer.
    ///
    /// # Safety
    /// The returned pointer is only valid if the original pointer passed to `new` is still valid.
    #[inline(always)]
    pub fn ptr(&self) -> *mut T {
        (self.ptr & Self::PTR_MASK) as *mut T
    }
    
    /// Get a reference to the pointed value.
    ///
    /// # Safety
    /// The caller must ensure that:
    /// - The pointer is valid and points to a properly initialized `T`
    /// - The pointed-to value is not being concurrently mutated
    #[inline(always)]
    pub unsafe fn as_ref(&self) -> &T {
        unsafe { &*self.ptr() }
    }

    /// Get a mutable reference to the pointed value.
    ///
    /// # Safety
    /// The caller must ensure that:
    /// - The pointer is valid and points to a properly initialized `T`
    /// - No other references to the pointed-to value exist
    #[inline(always)]
    pub unsafe fn as_mut(&mut self) -> &mut T {
        unsafe { &mut *self.ptr() }
    }
    
    /// Check if the pointer is null (ignoring the tag)
    #[inline(always)]
    pub fn is_null(&self) -> bool {
        self.ptr() as usize == 0
    }
}

// Safety: TaggedPtr is Send/Sync if T is Send/Sync
unsafe impl<T: Send> Send for TaggedPtr<T> {}
unsafe impl<T: Sync> Sync for TaggedPtr<T> {}

impl<T> Clone for TaggedPtr<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for TaggedPtr<T> {}

impl<T> core::fmt::Debug for TaggedPtr<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TaggedPtr")
            .field("tag", &self.tag())
            .field("ptr", &format_args!("{:p}", self.ptr()))
            .finish()
    }
}

/// Allocator trait for arena-allocated tagged pointers.
///
/// This trait should be implemented by arena allocators to enable
/// the arena version of tagged dispatch.
///
/// # Example
///
/// ```rust
/// use tagged_dispatch::TaggedAllocator;
/// use bumpalo::Bump;
///
/// // Bumpalo automatically implements TaggedAllocator when the feature is enabled
/// let arena = Bump::new();
/// let ptr = arena.alloc(42);
/// ```
pub trait TaggedAllocator {
    /// Allocate space for a value and return a pointer to it.
    ///
    /// The allocated memory should have the same lifetime as the allocator.
    fn alloc<T>(&self, value: T) -> *mut T;
}

// Implement TaggedAllocator for common arena allocators when their features are enabled

#[cfg(feature = "bumpalo")]
impl TaggedAllocator for bumpalo::Bump {
    #[inline]
    fn alloc<T>(&self, value: T) -> *mut T {
        bumpalo::Bump::alloc(self, value) as *mut T
    }
}

// Note: typed_arena doesn't implement TaggedAllocator directly
// because it can only allocate values of a single type T.
// Instead, the arena builder pattern generates separate arenas
// for each variant type when typed_arena is enabled.

/// Statistics for arena memory usage.
#[derive(Debug, Clone, Copy, Default)]
pub struct ArenaStats {
    /// Total bytes currently allocated
    pub allocated_bytes: usize,
    /// Total capacity of all chunks
    pub chunk_capacity: usize,
}

/// Trait for arena builders generated by the macro.
///
/// Provides memory management capabilities for arena-allocated
/// tagged dispatch types.
pub trait ArenaBuilder<'a>: Sized {
    /// Create a new builder with default settings.
    ///
    /// When both allocators are available, this prefers bumpalo
    /// for its superior flexibility.
    fn new() -> Self;

    /// Reset all allocations, invalidating existing references.
    ///
    /// # Safety
    ///
    /// This invalidates all references previously allocated from this builder.
    /// Using any such references after reset is undefined behavior.
    fn reset(&mut self);

    /// Clear allocations and attempt to reclaim memory.
    ///
    /// More aggressive than reset, this tries to return memory to the OS.
    fn clear(&mut self);

    /// Get current memory usage statistics.
    fn stats(&self) -> ArenaStats;
}

/// A simple box allocator for owned tagged pointers.
///
/// This is used internally by the owned version of tagged dispatch.
pub struct BoxAllocator;

impl TaggedAllocator for BoxAllocator {
    #[inline]
    fn alloc<T>(&self, value: T) -> *mut T {
        Box::into_raw(Box::new(value))
    }
}

// Module with helper utilities
#[doc(hidden)]
pub mod __private {
    pub use core::mem;
    pub use core::ptr;
    pub use core::marker::PhantomData;
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_tag_extraction() {
        let ptr = core::ptr::null_mut::<u32>();
        let tagged = TaggedPtr::new(ptr, 127);
        assert_eq!(tagged.tag(), 127);
        assert_eq!(tagged.ptr(), ptr);
    }
    
    #[test]
    fn test_tag_preservation() {
        let value = Box::new(42u32);
        let ptr = Box::into_raw(value);
        
        for tag in 0..128u8 {
            let tagged = TaggedPtr::new(ptr, tag);
            assert_eq!(tagged.tag(), tag);
            assert_eq!(tagged.ptr(), ptr);
        }
        
        // Clean up
        unsafe { let _ = Box::from_raw(ptr); }
    }
    
    #[test]
    fn test_size() {
        assert_eq!(core::mem::size_of::<TaggedPtr<()>>(), 8);
    }
    
    #[test]
    #[should_panic(expected = "Tag must be less than 128")]
    fn test_tag_overflow() {
        let ptr = core::ptr::null_mut::<u32>();
        let _tagged = TaggedPtr::new(ptr, 128);
    }
    
    #[cfg(feature = "bumpalo")]
    #[test]
    fn test_bumpalo_allocator() {
        use bumpalo::Bump;
        
        let arena = Bump::new();
        let value = 42u32;
        let ptr = arena.alloc(value);
        
        // Should be able to create a tagged pointer with arena allocation
        let tagged = TaggedPtr::new(ptr, 5);
        assert_eq!(tagged.tag(), 5);
        unsafe {
            assert_eq!(*tagged.as_ref(), 42);
        }
    }
}
