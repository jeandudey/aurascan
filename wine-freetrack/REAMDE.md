# FreeTrack and NPClient Wine DLLs

This is a set of DLLs for providing the FreeTrack Client and NPClient DLLs.

Instead of using Windows shared memory primitives it just exposes the
symbols of the FreeTrack Client and NPClient DLLs and uses POSIX shared
memory.

The POSIX shared memory name is `/freetrack-shm` and uses the `flock`
system call to synchronize access to the memory region, it uses the
same `FTHeap` data structure and size for the region, although using
`flock` is not strictly necessary, it is recommended to acquire and
release the lock when writing.
