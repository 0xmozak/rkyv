#[cfg(not(feature = "std"))]
use alloc::{alloc::alloc, boxed::Box, rc, sync};
#[cfg(feature = "std")]
use std::{alloc::alloc, rc, sync};

use ptr_meta::Pointee;
use rancor::Fallible;

use crate::{
    de::{Metadata, Pooling, PoolingExt as _, SharedPointer},
    rc::{
        ArcFlavor, ArchivedRc, ArchivedRcWeak, RcFlavor, RcResolver,
        RcWeakResolver,
    },
    ser::{Sharing, Writer},
    Archive, ArchivePointee, ArchiveUnsized, Deserialize, DeserializeUnsized,
    Serialize, SerializeUnsized,
};

// Rc

impl<T: ArchiveUnsized + ?Sized> Archive for rc::Rc<T> {
    type Archived = ArchivedRc<T::Archived, RcFlavor>;
    type Resolver = RcResolver;

    #[inline]
    unsafe fn resolve(
        &self,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        ArchivedRc::resolve_from_ref(self.as_ref(), pos, resolver, out);
    }
}

impl<T, S> Serialize<S> for rc::Rc<T>
where
    T: SerializeUnsized<S> + ?Sized + 'static,
    S: Fallible + Writer + Sharing + ?Sized,
{
    #[inline]
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        ArchivedRc::<T::Archived, RcFlavor>::serialize_from_ref(
            self.as_ref(),
            serializer,
        )
    }
}

unsafe impl<T: ?Sized> SharedPointer<T> for rc::Rc<T> {
    unsafe fn from_value(ptr: *mut T) -> *mut T {
        let rc = rc::Rc::<T>::from(unsafe { Box::from_raw(ptr) });
        rc::Rc::into_raw(rc).cast_mut()
    }

    unsafe fn drop(ptr: *mut T) {
        drop(unsafe { rc::Rc::from_raw(ptr) });
    }
}

impl<T, D> Deserialize<rc::Rc<T>, D> for ArchivedRc<T::Archived, RcFlavor>
where
    T: ArchiveUnsized + Pointee + ?Sized + 'static,
    T::Archived: DeserializeUnsized<T, D>,
    T::Metadata: Into<Metadata>,
    Metadata: Into<T::Metadata>,
    D: Fallible + Pooling + ?Sized,
{
    #[inline]
    fn deserialize(&self, deserializer: &mut D) -> Result<rc::Rc<T>, D::Error> {
        let raw_shared_ptr = deserializer
            .deserialize_shared::<_, rc::Rc<T>, _>(
                self.get(),
                // TODO: make sure that Rc<()> and Arc<()> won't alloc with
                // zero size layouts
                |layout| unsafe { alloc(layout) },
            )?;
        unsafe {
            rc::Rc::<T>::increment_strong_count(raw_shared_ptr);
        }
        unsafe { Ok(rc::Rc::<T>::from_raw(raw_shared_ptr)) }
    }
}

impl<T: ArchivePointee + PartialEq<U> + ?Sized, U: ?Sized> PartialEq<rc::Rc<U>>
    for ArchivedRc<T, RcFlavor>
{
    #[inline]
    fn eq(&self, other: &rc::Rc<U>) -> bool {
        self.get().eq(other.as_ref())
    }
}

// rc::Weak

impl<T: ArchiveUnsized + ?Sized> Archive for rc::Weak<T> {
    type Archived = ArchivedRcWeak<T::Archived, RcFlavor>;
    type Resolver = RcWeakResolver;

    #[inline]
    unsafe fn resolve(
        &self,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        ArchivedRcWeak::resolve_from_ref(
            self.upgrade().as_ref().map(|v| v.as_ref()),
            pos,
            resolver,
            out,
        );
    }
}

impl<T, S> Serialize<S> for rc::Weak<T>
where
    T: SerializeUnsized<S> + ?Sized + 'static,
    S: Fallible + Writer + Sharing + ?Sized,
{
    #[inline]
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        ArchivedRcWeak::<T::Archived, RcFlavor>::serialize_from_ref(
            self.upgrade().as_ref().map(|v| v.as_ref()),
            serializer,
        )
    }
}

// Deserialize can only be implemented for sized types because weak pointers
// don't have from/into raw functions.
impl<T, D> Deserialize<rc::Weak<T>, D> for ArchivedRcWeak<T::Archived, RcFlavor>
where
    T: Archive + 'static,
    T::Archived: DeserializeUnsized<T, D>,
    D: Fallible + Pooling + ?Sized,
{
    #[inline]
    fn deserialize(
        &self,
        deserializer: &mut D,
    ) -> Result<rc::Weak<T>, D::Error> {
        Ok(match self {
            ArchivedRcWeak::None => rc::Weak::new(),
            ArchivedRcWeak::Some(r) => {
                rc::Rc::downgrade(&r.deserialize(deserializer)?)
            }
        })
    }
}

// Arc

impl<T: ArchiveUnsized + ?Sized> Archive for sync::Arc<T> {
    type Archived = ArchivedRc<T::Archived, ArcFlavor>;
    type Resolver = RcResolver;

    #[inline]
    unsafe fn resolve(
        &self,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        ArchivedRc::resolve_from_ref(self.as_ref(), pos, resolver, out);
    }
}

impl<T, S> Serialize<S> for sync::Arc<T>
where
    T: SerializeUnsized<S> + ?Sized + 'static,
    S: Fallible + Writer + Sharing + ?Sized,
{
    #[inline]
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        ArchivedRc::<T::Archived, ArcFlavor>::serialize_from_ref(
            self.as_ref(),
            serializer,
        )
    }
}

unsafe impl<T: ?Sized> SharedPointer<T> for sync::Arc<T> {
    unsafe fn from_value(ptr: *mut T) -> *mut T {
        let arc = sync::Arc::<T>::from(unsafe { Box::from_raw(ptr) });
        sync::Arc::into_raw(arc).cast_mut()
    }

    unsafe fn drop(ptr: *mut T) {
        drop(unsafe { sync::Arc::from_raw(ptr) });
    }
}

impl<T, D> Deserialize<sync::Arc<T>, D> for ArchivedRc<T::Archived, ArcFlavor>
where
    T: ArchiveUnsized + Pointee + ?Sized + 'static,
    T::Metadata: Into<Metadata>,
    Metadata: Into<T::Metadata>,
    T::Archived: DeserializeUnsized<T, D>,
    D: Fallible + Pooling + ?Sized,
{
    #[inline]
    fn deserialize(
        &self,
        deserializer: &mut D,
    ) -> Result<sync::Arc<T>, D::Error> {
        let raw_shared_ptr = deserializer
            .deserialize_shared::<_, sync::Arc<T>, _>(
                self.get(),
                // TODO: make sure that Rc<()> and Arc<()> won't alloc with
                // zero size layouts
                |layout| unsafe { alloc(layout) },
            )?;
        unsafe {
            sync::Arc::<T>::increment_strong_count(raw_shared_ptr);
        }
        unsafe { Ok(sync::Arc::<T>::from_raw(raw_shared_ptr)) }
    }
}

impl<T, U> PartialEq<sync::Arc<U>> for ArchivedRc<T, ArcFlavor>
where
    T: ArchivePointee + PartialEq<U> + ?Sized,
    U: ?Sized,
{
    #[inline]
    fn eq(&self, other: &sync::Arc<U>) -> bool {
        self.get().eq(other.as_ref())
    }
}

// sync::Weak

impl<T: ArchiveUnsized + ?Sized> Archive for sync::Weak<T> {
    type Archived = ArchivedRcWeak<T::Archived, ArcFlavor>;
    type Resolver = RcWeakResolver;

    #[inline]
    unsafe fn resolve(
        &self,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        ArchivedRcWeak::resolve_from_ref(
            self.upgrade().as_ref().map(|v| v.as_ref()),
            pos,
            resolver,
            out,
        );
    }
}

impl<T, S> Serialize<S> for sync::Weak<T>
where
    T: SerializeUnsized<S> + ?Sized + 'static,
    S: Fallible + Writer + Sharing + ?Sized,
{
    #[inline]
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        ArchivedRcWeak::<T::Archived, ArcFlavor>::serialize_from_ref(
            self.upgrade().as_ref().map(|v| v.as_ref()),
            serializer,
        )
    }
}

// Deserialize can only be implemented for sized types because weak pointers
// don't have from/into raw functions.
impl<T, D> Deserialize<sync::Weak<T>, D>
    for ArchivedRcWeak<T::Archived, ArcFlavor>
where
    T: Archive + 'static,
    T::Archived: DeserializeUnsized<T, D>,
    D: Fallible + Pooling + ?Sized,
{
    #[inline]
    fn deserialize(
        &self,
        deserializer: &mut D,
    ) -> Result<sync::Weak<T>, D::Error> {
        Ok(match self {
            ArchivedRcWeak::None => sync::Weak::new(),
            ArchivedRcWeak::Some(r) => {
                sync::Arc::downgrade(&r.deserialize(deserializer)?)
            }
        })
    }
}
