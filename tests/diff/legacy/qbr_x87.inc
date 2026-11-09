// Verbatim copy of the x87 rounding functions from legacy internal/c/libqb.cpp
// (commit 545b859, lines 161-210). Test-only reference oracle for diff_round.cpp;
// production code now uses crates/qb-runtime/src/round.rs.
    //QBASIC compatible rounding via FPU:
    //FLDS=load single
    //FLDL=load double
    //FLDT=load long double
    int64 qbr(long double f){
        int64 i; int temp=0;
        if (f>9223372036854775807) {temp=1;f=f-9223372036854775808u;} //if it's too large for a signed int64, make it an unsigned int64 and return that value if possible.
        __asm__ (
            "fldt %1;"
            "fistpll %0;"              
            :"=m" (i)
            :"m" (f)
        );
        if (temp) return i|0x8000000000000000;// if it's an unsigned int64, manually set the bit flag
        return i;
    }
    uint64 qbr_longdouble_to_uint64(long double f){
        uint64 i;
        __asm__ (
            "fldt %1;"
            "fistpll %0;"              
            :"=m" (i)
            :"m" (f)
        );
        return i;
    }
    int32 qbr_float_to_long(float f){
        int32 i;
        __asm__ (
            "flds %1;"
            "fistpl %0;"              
            :"=m" (i)
            :"m" (f)
        );
        return i;
    }
    int32 qbr_double_to_long(double f){
        int32 i;
        __asm__ (
            "fldl %1;"
            "fistpl %0;"              
            :"=m" (i)
            :"m" (f)
        );
        return i;
    }
    void fpu_reinit(){
        unsigned int mode = 0x37F;
        asm("fldcw %0" : : "m" (*&mode));
    }
