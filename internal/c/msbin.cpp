// msbin.cpp — migrated from C to C++ (originally msbin.c)
//
// Implementations of Microsoft Binary Format (MBF) <-> IEEE floating point conversion.
//
// Functions:
//     _fmsbintoieee()  — convert 4-byte MBF float to IEEE single
//     _fieeetomsbin()  — convert IEEE single to 4-byte MBF float
//     _dmsbintoieee()  — convert 8-byte MBF double to IEEE double
//     _dieeetomsbin()  — convert IEEE double to 8-byte MBF double
//
// These functions do not handle IEEE NaNs or infinities.
// IEEE denormals are treated as zeros.
//
// Return: 0 on success, 1 on overflow.
//

#include <cstring>   // memcpy, memset
#include <cstdint>   // uint8_t, uint32_t

// Use the project type aliases (provided by common.h / os.h when included via libqb)
// A forward declaration guard lets this file also compile standalone.
#ifndef INC_COMMON_CPP
    using int32  = int;
    using uint32 = unsigned int;
#endif

// -----------------------------------------------------------------------
// _fmsbintoieee — Microsoft Binary Format (4-byte) → IEEE single
// -----------------------------------------------------------------------
int32 _fmsbintoieee(float *src4, float *dest4)
{
    const unsigned char *msbin = reinterpret_cast<const unsigned char *>(src4);
    unsigned char       *ieee  = reinterpret_cast<unsigned char *>(dest4);

    // MS Binary Format:  m3 | m2 | m1 | exponent
    //   m1 (most significant) = sbbb|bbbb   s=sign, b=mantissa bit
    // IEEE Single:        seee|eeee emmm|mmmm mmmm|mmmm mmmm|mmmm

    const unsigned char sign = msbin[2] & 0x80u;

    ieee[0] = ieee[1] = ieee[2] = ieee[3] = 0;

    // Exponent of zero means zero in MBF
    if (msbin[3] == 0) return 0;

    ieee[3] |= sign;

    // MBF bias=128, decimal before assumed bit
    // IEEE bias=127, decimal after  assumed bit  → subtract 2
    const unsigned char ieee_exp = msbin[3] - 2u;

    ieee[3] |= ieee_exp >> 1;         // top 7 bits of exponent
    ieee[2] |= ieee_exp << 7;         // lowest exponent bit
    ieee[2] |= msbin[2] & 0x7Fu;     // mantissa (mask out MBF sign)
    ieee[1]  = msbin[1];
    ieee[0]  = msbin[0];

    return 0;
}

// -----------------------------------------------------------------------
// _fieeetomsbin — IEEE single → Microsoft Binary Format (4-byte)
// -----------------------------------------------------------------------
int32 _fieeetomsbin(float *src4, float *dest4)
{
    const unsigned char *ieee  = reinterpret_cast<const unsigned char *>(src4);
    unsigned char       *msbin = reinterpret_cast<unsigned char *>(dest4);

    const unsigned char sign = ieee[3] & 0x80u;
    unsigned char msbin_exp  = 0;
    msbin_exp |= ieee[3] << 1;
    msbin_exp |= ieee[2] >> 7;

    // Exponent 0xFE overflows in MBF
    if (msbin_exp == 0xFEu) return 1;

    msbin_exp += 2u; // -127 + 128 + 1

    msbin[0] = msbin[1] = msbin[2] = msbin[3] = 0;

    msbin[3]  = msbin_exp;
    msbin[2] |= sign;
    msbin[2] |= ieee[2] & 0x7Fu;
    msbin[1]  = ieee[1];
    msbin[0]  = ieee[0];

    return 0;
}

// -----------------------------------------------------------------------
// _dmsbintoieee — Microsoft Binary Format (8-byte) → IEEE double
// -----------------------------------------------------------------------
int32 _dmsbintoieee(double *src8, double *dest8)
{
    unsigned char  msbin[8];
    unsigned char *ieee = reinterpret_cast<unsigned char *>(dest8);

    memcpy(msbin, src8, 8);

    // MS Binary Format (8-byte):  m7|m6|m5|m4|m3|m2|m1|exponent
    //   m1 = smmm|mmmm
    // IEEE Double: seee|eeee eeee|mmmm mmmm|mmmm ... (52-bit mantissa)

    const unsigned char sign = msbin[6] & 0x80u;

    ieee[0]=ieee[1]=ieee[2]=ieee[3]=ieee[4]=ieee[5]=ieee[6]=ieee[7]=0;

    if (msbin[7] == 0) return 0; // zero

    ieee[7] |= sign;

    // MBF bias=128, IEEE bias=1023 → +1023-128-1 = +894
    const uint32 ieee_exp = static_cast<uint32>(msbin[7]) - 128u - 1u + 1023u;

    ieee[7] |= static_cast<unsigned char>(ieee_exp >> 4);   // top 4 bits of exponent
    ieee[6] |= static_cast<unsigned char>(ieee_exp << 4);   // low 4 bits of exponent

    // Shift MBF mantissa right by 1 bit (bytes are in reverse order)
    for (int i = 6; i > 0; --i) {
        msbin[i] <<= 1;
        msbin[i] |= msbin[i-1] >> 7;
    }
    msbin[0] <<= 1;

    // Place mantissa into IEEE array
    for (int i = 6; i > 0; --i) {
        ieee[i] |= msbin[i] >> 4;
        ieee[i-1] |= msbin[i] << 4;
    }
    ieee[0] |= msbin[0] >> 4;

    // Check for mantissa overflow (IEEE has half a byte less)
    return (msbin[0] & 0x0Fu) ? 1 : 0;
}

// -----------------------------------------------------------------------
// _dieeetomsbin — IEEE double → Microsoft Binary Format (8-byte)
// -----------------------------------------------------------------------
int32 _dieeetomsbin(double *src8, double *dest8)
{
    unsigned char  ieee[8];
    unsigned char *msbin = reinterpret_cast<unsigned char *>(dest8);

    memcpy(ieee, src8, 8);

    memset(msbin, 0, sizeof(double));

    // Check for zero
    unsigned char any_on = 0;
    for (int i = 0; i < 8; ++i) any_on |= ieee[i];
    if (!any_on) return 0;

    const unsigned char sign = ieee[7] & 0x80u;
    msbin[6] |= sign;

    uint32 msbin_exp = (static_cast<uint32>(ieee[7] & 0x7Fu) << 4u);
    msbin_exp += ieee[6] >> 4u;

    // Verify exponent fits in MBF 8-bit field
    msbin_exp = msbin_exp - 0x3FFu + 0x80u + 1u;
    if ((msbin_exp & 0xFF00u) != 0) return 1; // overflow

    msbin[7] = static_cast<unsigned char>(msbin_exp);

    // Shift IEEE mantissa up by 3 bits into MBF format
    ieee[6] &= 0x0Fu; // mask out exponent bits
    for (int i = 6; i > 0; --i) {
        msbin[i] |= ieee[i] << 3;
        msbin[i] |= ieee[i-1] >> 5;
    }
    msbin[0] |= ieee[0] << 3;

    return 0;
}
