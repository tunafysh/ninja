#pragma author Hannan Smani
#pragma magic [48 53 43 47]
#pragma endian little
#pragma MIME application/x-shuriken

import std.mem;

struct Shuriken {
    char magic_bytes[4];
    u16 metadata_length @ 0x4;
    char metadata[metadata_length] @ 0x6;
    u8 archive[while(!std::mem::eof())] @ 0x6 + metadata_length;
};


// Map the top-level struct at the start of the file
Shuriken shuriken @ 0x0;