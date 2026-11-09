// Verbatim copy of legacy internal/c/libqb.cpp (commit 4aa6f1f; this code is unchanged from the oracle 545b859), lines 3543-3594.
// Test-only reference oracle for tests/diff/diff_gfx.cpp; production code is crates/qb-runtime/src/gfx.
static std::vector<uint8> cblend_storage;
static std::vector<uint8> ablend_storage;
uint8 *cblend=NULL;
uint8 *ablend=NULL;
uint8 *ablend127;
uint8 *ablend128;
//to save 16MB of RAM, software blend tables are only allocated if a 32-bit image is created
void init_blend(){
    uint8 *cp;
    int32 i,x2,x3,i2,z;
    float f,f2,f3;
    cblend_storage.resize(16777216);
    cblend=cblend_storage.data();
    cp=cblend;
    for (i=0;i<256;i++){//source alpha
        for (x2=0;x2<256;x2++){//source
            for (x3=0;x3<256;x3++){//dest
                f=i;
                f2=x2;
                f3=x3;
                f/=255.0;//0.0-1.0
                *cp++=qbr_float_to_long((f*f2)+((1.0-f)*f3));//CINT(0.0-255.0)
            }}}
            /*
                "60%+60%=84%" formula
                imagine a 60% opaque lens, you can see 40% of whats behind
                now put another 60% opaque lens on top of it
                you can now see 40% of the previous lens of which 40% is of the original scene
                40% of 40% is 16%
                100%-16%=84%
                V1=60, V2=60
                v1=V1/100, v2=V2/100
                iv1=1-v1, iv2=1-v2
                iv3=iv1*iv2
                v3=1-iv3
                V3=v3*100
            */
            ablend_storage.resize(65536);
            ablend=ablend_storage.data();
            cp=ablend;
            for (i=0;i<256;i++){//first alpha value
                for (i2=0;i2<256;i2++){//second alpha value
                    f=i; f2=i2;
                    f/=255.0; f2/=255.0;
                    f=1.0-f; f2=1.0-f2;
                    f3=f*f2;
                    z=qbr_float_to_long((1.0-f3)*255.0);
                    *cp++=z;
                }}
                ablend127=ablend+(127<<8);
                ablend128=ablend+(128<<8);
}
