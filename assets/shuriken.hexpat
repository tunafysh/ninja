// this file is for use with ImHex. idk about any other application that supports this format so i take no responsibility for anything going wrong :D
#pragma author tunafysh
#pragma magic [48 53 52 5A 45 47]
#pragma endian little
#pragma MIME application/x-shuriken

struct Shuriken {
    char magic_bytes[6];
    u16 metadata_length @ 0x6;
    char metadata[metadata_length] @ 0x8;
    u32 archive_length @ 0x8 + metadata_length;
    u8 archive[archive_length] @ 0x8 + metadata_length + 0x4;
    u8 signature[32] @ 0x8 + metadata_length + 0x4 + archive_length;
};


// Map the top-level struct at the start of the file
Shuriken shuriken @ 0x0;