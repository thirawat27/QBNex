// Verbatim copy of legacy internal/c/libqb.cpp (commit 4aa6f1f; this code is unchanged from the oracle 545b859), lines 16685-16707.
// Test-only reference oracle for tests/diff_gfx.cpp; production code is internal/c/qbrt/src/gfx.
    float func_pmap(float val,int32 option){
        static int32 x,y;
        if (new_error) return 0;
        if (!write_page->text){
            //note: for QBASIC/4.5/7.1 compatibility clipping_or_scaling check is skipped
            if (option==0){
                x=qbr_float_to_long(val*write_page->scaling_x+write_page->scaling_offset_x);
                return x;
            }
            if (option==1){
                y=qbr_float_to_long(val*write_page->scaling_y+write_page->scaling_offset_y);
                return y;
            }
            if (option==2){
                return (((double)qbr_float_to_long(val))-write_page->scaling_offset_x)/write_page->scaling_x;
            }
            if (option==3){
                return (((double)qbr_float_to_long(val))-write_page->scaling_offset_y)/write_page->scaling_y;
            }
        }//!write_page->text
        error(5);
        return 0;
    }
