# Called only from native code, which R8 can't see.
-keep, includedescriptorclasses class org.rustls.platformverifier.** { *; }
