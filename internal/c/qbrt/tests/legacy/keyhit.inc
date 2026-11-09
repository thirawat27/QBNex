// VERBATIM copies from the legacy libqb.cpp (oracle 545b859), test-only:
//  - keyhit globals (lines 1573-1579)
//  - the keyhit push sequence of keydown() (lines 30612-30619), wrapped in a function by diff_keybuf.cpp
//  - func__keyhit (lines 26031-26049)
//keyhit cyclic buffer
int64 keyhit[8192];
//    keyhit specific internal flags: (stored in high 32-bits)
//    &4294967296->numpad was used
int32 keyhit_nextfree=0;
int32 keyhit_next=0;
//note: if full, the oldest message is discarded to make way for the new message
static void legacy_keyhit_push(uint32 x2,int64 numpadkey){
        //add x2 to keyhit buffer
        static int32 z;
        z=(keyhit_nextfree+1)&0x1FFF;
        if (z==keyhit_next){//remove oldest message when cyclic buffer is full
            keyhit_next=(keyhit_next+1)&0x1FFF;
        }
        keyhit[keyhit_nextfree]=x2|numpadkey;
        keyhit_nextfree=z;
}
        int32 func__keyhit(){
            /*
                //keyhit cyclic buffer
                int64 keyhit[8192];
                //    keyhit specific internal flags: (stored in high 32-bits)
                //    &4294967296->numpad was used
                int32 keyhit_nextfree=0;
                int32 keyhit_next=0;
                //note: if full, the oldest message is discarded to make way for the new message
            */
            if (keyhit_next!=keyhit_nextfree){
                static int32 x;
                x=*(int32*)&keyhit[keyhit_next];
                keyhit_next=(keyhit_next+1)&0x1FFF;
                return x;
            }
            return 0;
        }
        
