# lib3h_persistence_impl

Persistence implementations for lib3h and holochain. Provides content addressable storage (CAS) and entity attribute value (index) associative stores in three flavors.

 - a pure, thread safe in memory storage
 - file based storage using hiearchical directories to associate and query data
 - [pickledb](https://github.com/seladb/pickledb-rs) for database backed storage

