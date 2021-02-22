storage-poc aims at exploring the usage of custom Storages, rather than custom Allocators.

#   Goals

This is a Proof-of-Concept aiming at:

-   Demonstrating the technical feasibility.
-   Showcasing the usage of storages for various collections, with different requirements.
-   Sketching out a potential API.

This experiment does not intend to provide a production-ready solution.


#   Why not Allocators?

The currently proposed API for allocators is centered around `NonNull`, that is a pointer.

Creating a `Box` based on a custom allocator means storing the pointer within the `Box`. If the allocator were to ever
move, the pointer within the `Box` would become dangling.

This limitation prevents storing the content of the `Box`, and other collections, _inline_. And there are many benefits
to doing so:

-   Imagine returning a `Box<dyn Future<_>, InlineStorage>`: abstract type, dynamically dispatched, without memory
    allocation.
-   Imagine storing `Box<dyn FnOnce(), InlineStorage>`: you can build a task queue, with no memory allocation in sight.
-   Imagine creating a `const LOOKUP: BTreeMap<K, V, InlineStorage> = /**/;`: no memory allocation, hence `const` able.

Abstracting away pointers may even allow storing those collections in shared memory.

Interested? Continue reading!


#   How to navigate?

The repository contains 3 parts:

-   [`traits.rs`](src/traits.rs) sketches out the API of the necessary storage traits.
-   [`collections`](src/collections) sketches out how to adapt a few known collections with disparate needs to
    demonstrate the usage of the traits, in practice.
-   The other modules are implementations of the traits:
    -   [`allocator.rs`](src/allocator.rs) implementations simply adapt an Allocator.
    -   [`inline.rs`](src/inline.rs) implementations store everything _inline_.
    -   [`alternative.rs`](src/alternative.rs) combines 2 storages, either of which is active at the moment. It is used
        to implement the [`small.rs`](src/small.rs) family of storages.
    -   [`fallback.rs`](src/fallback.rs) combines 2 storages, using both simultaneously, with a preference for the
        first -- which should be cheaper.


#   What is the API?

The [`traits.rs`](src/traits.rs) defines an API around 2 axes:

-   Single vs Multi: whether the storage allocates a single item at a time, or allows juggling multiple items.
-   Element vs Range: whether the storage allocates a single element at a time, or allocates a range of elements.

The API was created to be higher-level than the Allocator API. Being higher-level makes it easier to use, and allows
optimizations in the implementation.

The `SingleElementStorage` is (stripped down):

```rust
pub trait SingleElementStorage : ElementStorage {
    fn create<T: Pointee>(&mut self, value: T) -> Result<Self::Handle<T>, T>;

    fn allocate<T: ?Sized + Pointee>(&mut self, meta: T::MetaData) -> Result<Self::Handle<T>, AllocError>;
}

pub trait ElementStorage {
    type Handle<T: ?Sized + Pointee> : Clone + Copy;

    unsafe fn destroy<T: ?Sized + Pointee>(&mut self, handle: Self::Handle<T>);

    unsafe fn deallocate<T: ?Sized + Pointee>(&mut self, handle: Self::Handle<T>);

    unsafe fn get<T: ?Sized + Pointee>(&self, handle: Self::Handle<T>) -> NonNull<T>;
}
```

The `Handle<T>` is the magic allowing inline storage: they are something to be converted to an ephemeral pointer via
`storage.get(handle)`, but not a pointer themselves, and therefore supports relocations of the underlying storage.

From there, the lifecyle is rather obvious: `create`, `get` a few times, and finally `destroy`.

_Note: the `Pointee` trait comes from RFC2580, as implemented in [rfc2580](https://github.com/matthieu-m/rfc2580), it's
essentially a trait to split a pointer into pointer-to-data and meta-data and put it back together later._

And the `SingleRangeStorage` is (stripped down):

```rust
pub trait SingleRangeStorage : RangeStorage {
    fn allocate<T>(&mut self, capacity: Self::Capacity) -> Result<Self::Handle<T>, AllocError>;
}

pub trait RangeStorage {
    type Handle<T> : Clone + Copy;

    type Capacity : Capacity;

    fn maximum_capacity<T>(&self) -> Self::Capacity;

    unsafe fn deallocate<T>(&mut self, handle: Self::Handle<T>);

    unsafe fn get<T>(&self, handle: Self::Handle<T>) -> NonNull<[MaybeUninit<T>]>;

    unsafe fn try_grow<T>(&mut self, _handle: Self::Handle<T>, _new_capacity: Self::Capacity) -> Result<Self::Handle<T>, AllocError> {
        Err(AllocError)
    }

    unsafe fn try_shrink<T>(&mut self, _handle: Self::Handle<T>, _new_capacity: Self::Capacity) -> Result<Self::Handle<T>, AllocError> {
        Err(AllocError)
    }
}
```

Once again a `Handle<T>` to allow inline storage, and this time a `Capacity` to allow using a smaller type than `usize`
when possible.

The trait is lower-level, as it makes no assumption about which portions of the range are initialized or not, leaving
it up to the caller. This is necessary since `Vec` and `VecDeque` have different invariants here.


#   Can we replace the `std` collections tomorrow?

There is one blocker currently: the custom `Box` implementation is not coercible.

`CoerceUnsized` and `Unsize` are locked-down, and the custom implementation of RFC2580 therefore cannot implement
`CoerceUnsized` to be able to unsize `SizedMetadata<[u8; 3]>` into `SliceMetadata<[u8]>`. This in turn prevents `Box`
from being coercible. Oops.

For the purpose of this Proof-of-Concept, a stand-in `coerce` method has been added to the `ElementStorage` trait where
the implementation is essentially to go from handle to pointer, let the compiler coerce it, and then go back to handle
from there. Then a similar `coerce` method is implemented on `Box`, and things just work.

Then, there is the issue that the implementation relies on Generic Associated Types, which are quite unstable, though
good enough that the code compiles and runs without issues on nightly... as long as one doesn't try to coerce-unsize
the `Box`. It may be prudent to wait for the go-ahead of the compiler developers before switching any collection.


#   That's all folks!

And thanks for reading.
