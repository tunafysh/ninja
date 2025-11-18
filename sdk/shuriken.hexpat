// this file is for use with ImHex. idk about any other applicationthat supports this format so i take no responsibility for anything going wrong :D
#pragma author Hannan Smani
#pragma magic [48 53]
#pragma endian little
#pragma MIME application/x-shuriken

import std.mem;

struct Shuriken {
    char magic_bytes[2];
    u16 metadata_length @ 0x2;
    char metadata[metadata_length] @ 0x4;
    u8 archive[while(!std::mem::eof())] @ 0x4 + metadata_length;
};


// Map the top-level struct at the start of the file
Shuriken shuriken @ 0x0;