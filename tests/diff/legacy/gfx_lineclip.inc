// Verbatim copy of legacy internal/c/libqb.cpp (commit 4aa6f1f; this code is unchanged from the oracle 545b859), lines 7871-8013.
// Test-only reference oracle for tests/diff/diff_gfx.cpp; production code is crates/qb-runtime/src/gfx.
int32 lineclip_draw;//1=draw, 0=don't draw
int32 lineclip_x1,lineclip_y1,lineclip_x2,lineclip_y2;
int32 lineclip_skippixels;//the number of pixels from x1,y1 which won't be drawn

void lineclip(int32 x1,int32 y1,int32 x2,int32 y2,int32 xmin,int32 ymin,int32 xmax,int32 ymax){
    static double mx,my,y,x,d;
    static int32 xdis,ydis;
    lineclip_skippixels=0;
    
    
    if (x1>=xmin){ if (x1<=xmax){ if (y1>=ymin){ if (y1<=ymax){//(x1,y1) onscreen?
        if (x1==x2) if (y1==y2) goto singlepoint;//is it a single point? (needed to avoid "division by 0" errors)
        goto gotx1y1;
    }}}}
    
    //(x1,y1) offscreen...
    
    if (x1==x2) if (y1==y2){lineclip_draw=0; return;}//offscreen single point
    
    //ignore entirely offscreen lines requiring no further calculations
    if (x1<xmin) if (x2<xmin){lineclip_draw=0; return;}
    if (x1>xmax) if (x2>xmax){lineclip_draw=0; return;}
    if (y1<ymin) if (y2<ymin){lineclip_draw=0; return;}
    if (y1>ymax) if (y2>ymax){lineclip_draw=0; return;}
    
    mx=(x2-x1)/fabs((double)(y2-y1)); my=(y2-y1)/fabs((double)(x2-x1));
    //right wall from right
    if (x1>xmax){
        if (mx<0){
            y=(double)y1+((double)x1-(double)xmax)*my;
            if (y>=ymin){ if (y<=ymax){
                //double space indented values calculate pixels to skip
                xdis=x1; ydis=y1;
                x1=xmax; y1=qbr_float_to_long(y);
                xdis=abs(xdis-x1); ydis=abs(ydis-y1);
                if (xdis>=ydis) lineclip_skippixels=xdis; else lineclip_skippixels=ydis;
                goto gotx1y1;
            }}
        }
    }
    //left wall from left
    if (x1<xmin){
        if (mx>0){
            y=(double)y1+((double)xmin-(double)x1)*my;
            if (y>=ymin){ if (y<=ymax){
                //double space indented values calculate pixels to skip
                xdis=x1; ydis=y1;
                x1=xmin; y1=qbr_float_to_long(y);
                xdis=abs(xdis-x1); ydis=abs(ydis-y1);
                
                if (xdis>=ydis) lineclip_skippixels=xdis; else lineclip_skippixels=ydis;
                goto gotx1y1;
            }}
        }
    }
    //top wall from top
    if (y1<ymin){
        if (my>0){
            x=(double)x1+((double)ymin-(double)y1)*mx;
            if (x>=xmin){ if (x<=xmax){
                //double space indented values calculate pixels to skip
                xdis=x1; ydis=y1;
                x1=qbr_float_to_long(x); y1=ymin;
                xdis=abs(xdis-x1); ydis=abs(ydis-y1);
                if (xdis>=ydis) lineclip_skippixels=xdis; else lineclip_skippixels=ydis;
                goto gotx1y1;
            }}
        }
    }
    //bottom wall from bottom
    if (y1>ymax){
        if (my<0){
            x=(double)x1+((double)y1-(double)ymax)*mx;
            if (x>=xmin){ if (x<=xmax){
                //double space indented values calculate pixels to skip
                xdis=x1; ydis=y1;
                x1=qbr_float_to_long(x); y1=ymax;
                xdis=abs(xdis-x1); ydis=abs(ydis-y1);
                if (xdis>=ydis) lineclip_skippixels=xdis; else lineclip_skippixels=ydis;
                goto gotx1y1;
            }}
        }
    }
    lineclip_draw=0;
    return;
    gotx1y1:
    
    if (x2>=xmin){ if (x2<=xmax){ if (y2>=ymin){ if (y2<=ymax){
        goto gotx2y2;
    }}}}
    
    
    mx=(x1-x2)/fabs((double)(y1-y2)); my=(y1-y2)/fabs((double)(x1-x2));
    //right wall from right
    if (x2>xmax){
        if (mx<0){
            y=(double)y2+((double)x2-(double)xmax)*my;
            if (y>=ymin){ if (y<=ymax){
                x2=xmax; y2=qbr_float_to_long(y);
                goto gotx2y2;
            }}
        }
    }
    //left wall from left
    if (x2<xmin){
        if (mx>0){
            y=(double)y2+((double)xmin-(double)x2)*my;
            if (y>=ymin){ if (y<=ymax){
                x2=xmin; y2=qbr_float_to_long(y);
                goto gotx2y2;
            }}
        }
    }
    //top wall from top
    if (y2<ymin){
        if (my>0){
            x=(double)x2+((double)ymin-(double)y2)*mx;
            if (x>=xmin){ if (x<=xmax){
                x2=qbr_float_to_long(x); y2=ymin;
                goto gotx2y2;
            }}
        }
    }
    //bottom wall from bottom
    if (y2>ymax){
        if (my<0){
            x=(double)x2+((double)y2-(double)ymax)*mx;
            if (x>=xmin){ if (x<=xmax){
                x2=qbr_float_to_long(x); y2=ymax;
                goto gotx2y2;
            }}
        }
    }
    lineclip_draw=0;
    return;
    gotx2y2:
    singlepoint:
    lineclip_draw=1;
    lineclip_x1=x1; lineclip_y1=y1; lineclip_x2=x2; lineclip_y2=y2;
    
    
    return;
}
