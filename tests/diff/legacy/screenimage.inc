// Verbatim copy of legacy libqb.cpp (commit 545b859) lines 24949-25037: func__screenimage. Test-only.
        #ifdef DEPENDENCY_SCREENIMAGE
            int32 func__screenimage(int32 x1,int32 y1,int32 x2,int32 y2,int32 passed){
                
                #ifdef QBNex_WINDOWS
                    
                    static int32 init=0;
                    static int32 x,y,w,h;
                    
                    static HWND hwnd;
                    static RECT rect;
                    static HDC hdc;
                    
                    if (!init){
                        hwnd=GetDesktopWindow();
                        GetWindowRect(hwnd,&rect);
                        x=rect.right-rect.left; y=rect.bottom-rect.top;
                    }
                    
                    hdc=GetDC(NULL);
                    
                    static HDC hdc2=NULL;
                    static HBITMAP bitmap;
                    static int32 bx,by;
                    
                    if (passed){
                        if (x1<0) x1=0;
                        if (y1<0) y1=0;
                        if (x2>x-1) x2=x-1;
                        if (y2>y-1) y2=y-1;
                        w=x2-x1+1; h=y2-y1+1;
                        }else{
                        x1=0; x2=x-1; y1=0; y2=y-1; w=x; h=y;
                    }
                    
                    
                    
                    if (hdc2){
                        if ((w!=bx)||(h!=by)){
                            DeleteObject(bitmap);
                            ReleaseDC(NULL,hdc2);
                            hdc2=CreateCompatibleDC(hdc);
                            bitmap=CreateCompatibleBitmap(hdc,w,h); bx=w; by=h;
                            SelectObject(hdc2,bitmap);
                        }
                        }else{
                        hdc2=CreateCompatibleDC(hdc);
                        bitmap=CreateCompatibleBitmap(hdc,w,h); bx=w; by=h;
                        SelectObject(hdc2,bitmap);
                    }
                    
                    init=1;
                    
                    BitBlt(        hdc2, 
                    0,0, 
                    w,h,
                    hdc, 
                    x1,y1,
                    SRCCOPY);
                    
                    static BITMAPFILEHEADER   bmfHeader;  
                    static BITMAPINFOHEADER   bi;
                    bi.biSize = sizeof(BITMAPINFOHEADER);    
                    bi.biWidth = w;    
                    bi.biHeight = -h; //A bottom-up DIB is specified by setting the height to a positive number, while a top-down DIB is specified by setting the height to a negative number. The bitmap color table will be appended to the BITMAPINFO structure.
                    bi.biPlanes = 1;    
                    bi.biBitCount = 32;    
                    bi.biCompression = BI_RGB;    
                    bi.biSizeImage = 0;  
                    bi.biXPelsPerMeter = 0;    
                    bi.biYPelsPerMeter = 0;    
                    bi.biClrUsed = 0;    
                    bi.biClrImportant = 0;
                    
                    static int32 i,i2;
                    i2=func__dest();
                    i=func__newimage(w,h,32,1);
                    sub__dest(i);
                    
                    GetDIBits(hdc2,bitmap,0,h,write_page->offset,(BITMAPINFO*)&bi, DIB_RGB_COLORS);
                    sub__setalpha(255,NULL,NULL,NULL,0);
                    sub__dest(i2);
                    
                    ReleaseDC(NULL,hdc);
                    return i;
                    #else
                    return func__newimage(func_screenwidth(), func_screenheight(), 32, 1);
                #endif
            }
        #endif //DEPENDENCY_SCREENIMAGE
