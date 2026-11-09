// Verbatim copy of legacy internal/c/libqb.cpp (commit 4aa6f1f; this code is unchanged from the oracle 545b859), lines 11909-11988 12021-12115.
// Test-only reference oracle for tests/diff/diff_gfx.cpp; production code is crates/qb-runtime/src/gfx.
void qbg_sub_window(float x1,float y1,float x2,float y2,int32 passed){
    //                  &1
    //(passed&2)->SCREEN
    if (new_error) return;
    static float i;
    static float old_x,old_y;
    
    if (write_page->text) goto qbg_sub_window_error;
    if ((!(passed&1))&&(passed&2)) goto qbg_sub_window_error;//SCREEEN passed without any other arguements!
    
    //backup current qbg_x & qbg_y coordinates relative to viewport, not window
    if (write_page->clipping_or_scaling==2){
        old_x=write_page->x*write_page->scaling_x+write_page->scaling_offset_x;
        old_y=write_page->y*write_page->scaling_y+write_page->scaling_offset_y;
        }else{
        old_x=write_page->x;
        old_y=write_page->y;
    }
    
    
    if (passed&1){
        if (x1==x2) goto qbg_sub_window_error;
        if (y1==y2) goto qbg_sub_window_error;
        //sort so x1 & y1 contain the lower values
        if (x1>x2){i=x1; x1=x2; x2=i;}
        if (y1>y2){i=y1; y1=y2; y2=i;}
        if (!(passed&2)){
            i=y1; y1=y2; y2=i;
        }
        //Note: Window's coordinates are not based on prev. WINDOW values
        write_page->clipping_or_scaling=2;
        write_page->scaling_x=((float)(write_page->view_x2-write_page->view_x1))/(x2-x1);
        write_page->scaling_y=((float)(write_page->view_y2-write_page->view_y1))/(y2-y1);
        write_page->scaling_offset_x=-x1*write_page->scaling_x; //scaling offset should be applied before scaling
        write_page->scaling_offset_y=-y1*write_page->scaling_y;
        if (!(passed&2)){
            write_page->scaling_offset_y=-y2*write_page->scaling_y+(write_page->view_y2-write_page->view_y1);
        }
        write_page->window_x1=x1; write_page->window_x2=x2; write_page->window_y1=y1; write_page->window_y2=y2;
        
        
        if (x1==0){ if (y1==0){ if (x2==(write_page->width-1)){ if (y2==(write_page->height-1)){
            if ((write_page->scaling_x==1)&&(write_page->scaling_y==1)){
                if ((write_page->scaling_offset_x==0)&&(write_page->scaling_offset_y==0)){
                    goto qbg_sub_window_restore_default;
                }
            }
        }}}}
        
        //adjust qbg_x & qbg_y according to new window
        write_page->x=(old_x-write_page->scaling_offset_x)/write_page->scaling_x;
        write_page->y=(old_y-write_page->scaling_offset_y)/write_page->scaling_y;
        
        return;
        }else{
        //restore default WINDOW coordinates
        qbg_sub_window_restore_default:
        write_page->clipping_or_scaling=1;
        write_page->scaling_x=1;
        write_page->scaling_y=1;
        write_page->scaling_offset_x=0;
        write_page->scaling_offset_y=0;
        write_page->window_x1=0; write_page->window_x2=write_page->width-1; write_page->window_y1=0; write_page->window_y2=write_page->height-1;
        if (write_page->view_x1==0){ if (write_page->view_y1==0){ if (write_page->view_x2==(write_page->width-1)){ if (write_page->view_y2==(write_page->height-1)){
            if (write_page->view_offset_x==0){ if (write_page->view_offset_y==0){
                write_page->clipping_or_scaling=0;
            }}
        }}}}
        
        //adjust qbg_x & qbg_y according to new window
        write_page->x=old_x;
        write_page->y=old_y;
        
        return;
    }
    qbg_sub_window_error:
    error(5);
    return;
}

void qbg_sub_view(int32 x1,int32 y1,int32 x2,int32 y2,int32 fillcolor,int32 bordercolor,int32 passed){
    //   &1                                   &4              &8
    //    (passed&2)->coords_relative_to_screen
    if (new_error) return;
    //format: [{SCREEN}][(?,?)-(?,?)],[?],[?]
    //bordercolor draws a line AROUND THE OUTSIDE of the specified viewport
    //the current WINDOW settings do not affect inputted x,y values
    //the current VIEW settings do not affect inputted x,y values
    //REMEMBER! Recalculate WINDOW values based on new viewport dimensions
    int32 i;
    
    //PRE-ERROR CHECKING
    if (passed&1){
        if (x1<0) goto error;
        if (x1>=write_page->width) goto error;
        if (y1<0) goto error;
        if (y1>=write_page->height) goto error;
        if (x2<0) goto error;
        if (x2>=write_page->width) goto error;
        if (y2<0) goto error;
        if (y2>=write_page->height) goto error;
        }else{
        if (passed&2) goto error;
        if (passed&4) goto error;
        if (passed&8) goto error;
    }
    
    //reset DRAW attributes
    write_page->draw_ta=0.0; write_page->draw_scale=1.0;
    
    if (passed&1){
        //force x1,y1 to be the top left corner
        if (x2<x1){i=x1;x1=x2;x2=i;}
        if (y2<y1){i=y1;y1=y2;y2=i;}
        
        write_page->view_x1=x1; write_page->view_y1=y1; write_page->view_x2=x2; write_page->view_y2=y2;
        if ((passed&2)==0){
            write_page->view_offset_x=x1; write_page->view_offset_y=y1;
            }else{
            write_page->view_offset_x=0; write_page->view_offset_y=0;
        }
        if (!write_page->clipping_or_scaling) write_page->clipping_or_scaling=1;
        }else{
        //no argurments passed
        write_page->view_x1=0; write_page->view_y1=0; write_page->view_x2=write_page->width-1; write_page->view_y2=write_page->height-1;
        write_page->view_offset_x=0; write_page->view_offset_y=0;
        if (write_page->clipping_or_scaling==1) write_page->clipping_or_scaling=0;
    }
    
    //recalculate window values based on new viewport (if necessary)
    if (write_page->clipping_or_scaling==2){//WINDOW'ing in use
        write_page->scaling_x=((float)(write_page->view_x2-write_page->view_x1))/(write_page->window_x2-write_page->window_x1);
        write_page->scaling_y=((float)(write_page->view_y2-write_page->view_y1))/(write_page->window_y2-write_page->window_y1);
        write_page->scaling_offset_x=-write_page->window_x1*write_page->scaling_x;
        write_page->scaling_offset_y=-write_page->window_y1*write_page->scaling_y;
        if (write_page->window_y2<write_page->window_y1) write_page->scaling_offset_y=-write_page->window_y2*write_page->scaling_y+write_page->view_y2;
    }
    
    if (passed&4){//fillcolor
        qb32_boxfill(write_page->window_x1,write_page->window_y1,write_page->window_x2,write_page->window_y2,fillcolor);
    }
    
    if (passed&8){//bordercolor
        static int32 bx,by;
        by=write_page->view_y1-1;
        if ((by>=0)&&(by<write_page->height)){
            for (bx=write_page->view_x1-1;bx<=write_page->view_x2;bx++){
                if ((bx>=0)&&(bx<write_page->width)){
                    pset(bx,by,bordercolor);
                }}}
                by=write_page->view_y2+1;
                if ((by>=0)&&(by<write_page->height)){
                    for (bx=write_page->view_x1-1;bx<=write_page->view_x2;bx++){
                        if ((bx>=0)&&(bx<write_page->width)){
                            pset(bx,by,bordercolor);
                        }}}
                        bx=write_page->view_x1-1;
                        if ((bx>=0)&&(bx<write_page->width)){
                            for (by=write_page->view_y1-1;by<=write_page->view_y2;by++){
                                if ((by>=0)&&(by<write_page->height)){
                                    pset(bx,by,bordercolor);
                                }}}
                                bx=write_page->view_x2+1;
                                if ((bx>=0)&&(bx<write_page->width)){
                                    for (by=write_page->view_y1-1;by<=(write_page->view_y2+1);by++){
                                        if ((by>=0)&&(by<write_page->height)){
                                            pset(bx,by,bordercolor);
                                        }}}
    }
    
    return;
    error:
    error(5);
    return;
}
