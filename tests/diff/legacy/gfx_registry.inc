// Verbatim copy of legacy internal/c/libqb.cpp (oracle commit 545b859), lines 2865-2913 3592-3592 3712-3743 3895-4173.
// Test-only reference oracle for tests/diff/diff_gfx.cpp; production code is crates/qb-runtime/src/gfx.
#define IMG_BUFFERSIZE 4096
static std::unique_ptr<img_struct[]> qb_img_storage(new img_struct[IMG_BUFFERSIZE]());
static std::vector<std::vector<uint8>> qb_img_pixel_storage(IMG_BUFFERSIZE);
static std::vector<std::vector<uint32>> qb_img_palette_storage(IMG_BUFFERSIZE);
img_struct *img=qb_img_storage.get();
int32 nimg=IMG_BUFFERSIZE;
int32 nextimg=0;
static std::vector<std::vector<std::string>> qb_text_utf8_cells(IMG_BUFFERSIZE);
static std::vector<std::vector<uint8>> qb_text_utf8_cell_state(IMG_BUFFERSIZE);

static void qb_resize_img_owner_storage(int32 capacity){
    qb_img_pixel_storage.resize(capacity);
    qb_img_palette_storage.resize(capacity);
}

static void qb_set_img_external_offset(int32 index,uint8 *offset){
    std::vector<uint8>().swap(qb_img_pixel_storage[index]);
    img[index].offset=offset;
}

static void qb_set_img_external_palette(int32 index,uint32 *palette){
    std::vector<uint32>().swap(qb_img_palette_storage[index]);
    img[index].pal=palette;
}

static void qb_release_img_pixels(int32 index){
    std::vector<uint8>().swap(qb_img_pixel_storage[index]);
    img[index].offset=NULL;
}

static void qb_release_img_palette(int32 index){
    std::vector<uint32>().swap(qb_img_palette_storage[index]);
    img[index].pal=NULL;
}

static void qb_release_img_owned_resources(int32 index){
    if (img[index].flags&IMG_FREEMEM) qb_release_img_pixels(index);
    if (img[index].flags&IMG_FREEPAL) qb_release_img_palette(index);
}

static void qb_allocate_img_pixels(int32 index,ptrszint bytes){
    qb_img_pixel_storage[index].assign(static_cast<size_t>(bytes),0);
    img[index].offset=qb_img_pixel_storage[index].empty()?NULL:qb_img_pixel_storage[index].data();
}

static void qb_allocate_img_palette(int32 index){
    qb_img_palette_storage[index].assign(256,0);
    img[index].pal=qb_img_palette_storage[index].empty()?NULL:qb_img_palette_storage[index].data();
}
std::vector<uint32> fimg;//a list to recover freed indexes
int32 next_img_capacity(int32 required_slots){
    int32 capacity=nimg;
    if (capacity < IMG_BUFFERSIZE) capacity=IMG_BUFFERSIZE;
    while (capacity < required_slots){
        if (capacity > 1073741823){
            capacity=required_slots;
            break;
        }
        capacity*=2;
    }
    return capacity;
}

int32 grow_img_registry(int32 required_slots){
    if (required_slots <= nimg) return -1;
    const int32 previous_capacity=nimg;
    const int32 new_capacity=next_img_capacity(required_slots);
    std::unique_ptr<img_struct[]> resized_img(new(std::nothrow) img_struct[new_capacity]());
    if (!resized_img){
        error(502);
        return 0;
    }
    memcpy(resized_img.get(),img,previous_capacity*sizeof(img_struct));
    qb_img_storage.swap(resized_img);
    img=qb_img_storage.get();
    nimg=new_capacity;
    qb_resize_img_owner_storage(new_capacity);
    qb_text_utf8_cells.resize(new_capacity);
    qb_text_utf8_cell_state.resize(new_capacity);
    refresh_selected_pages();
    return -1;
}
uint32 newimg(){
    static int32 i;
    if (!fimg.empty()){
        i=fimg.back();
        fimg.pop_back();
        goto gotindex;
    }
    if (nextimg<nimg){
        i=nextimg++;
        goto gotindex;
    }
    if (!grow_img_registry(nextimg+1)) return 0;
    i=nextimg++;
    gotindex:
    img[i].valid=1;
    return i;
}

int32 freeimg(uint32 i){
    //returns: 0=failed, 1=success
    if (i>=nimg) return 0;
    if (!img[i].valid) return 0;
    if (img[i].lock_id){
        free_mem_lock((mem_lock*)img[i].lock_offset);//untag
    }
    qb_release_img_pixels(i);
    qb_release_img_palette(i);
    qb_release_text_utf8_cells(i);
    for (size_t page_index = 0; page_index < page.size(); ++page_index){
        if (page[page_index] == static_cast<int32>(i)) page[page_index] = 0;
    }
    memset(&img[i],0,sizeof(img_struct));
    fimg.push_back(i);
    return 1;
}


void imgrevert(int32 i){
    static int32 bpp;
    static img_struct *im;
    
    im=&img[i];
    bpp=im->compatible_mode;
    
    //revert to assumed default values
    im->bytes_per_pixel=1;
    im->font=16;
    im->color=15;
    im->print_mode=3;
    im->background_color=0;
    im->draw_ta=0.0; im->draw_scale=1.0;
    
    //revert to mode's set values
    switch (bpp){
        case 0:
        im->bits_per_pixel=16; im->bytes_per_pixel=2;
        im->color=7;
        im->text=1;
        im->cursor_show=0; im->cursor_firstvalue=4; im->cursor_lastvalue=4;
        break;
        case 1:
        im->bits_per_pixel=2;
        im->font=8;
        im->color=3;
        break;
        case 2:
        im->bits_per_pixel=1; 
        im->font=8;//it gets stretched from 8 to 16 later
        im->color=1;
        break;
        case 7:
        im->bits_per_pixel=4;
        im->font=8;
        break;
        case 8:
        im->bits_per_pixel=4;
        im->font=8;
        break;
        case 9:
        im->bits_per_pixel=4;
        im->font=14;
        break;
        case 10:
        im->bits_per_pixel=2;
        im->font=14;
        im->color=3;
        break;
        case 11:
        im->bits_per_pixel=1;
        im->color=1;
        break;
        case 12:
        im->bits_per_pixel=4;
        break;
        case 13:
        im->bits_per_pixel=8;
        im->font=8;
        break;
        case 256:
        im->bits_per_pixel=8;
        break;
        case 32:
        im->bits_per_pixel=32; im->bytes_per_pixel=4;
        im->color=0xFFFFFFFF;
        im->background_color=0xFF000000;
        break;
    };
    im->draw_color=im->color;
    
    //revert palette
    if (bpp!=32){
        restorepalette(im);
        im->transparent_color=-1;
    }
    
    //revert calculatable values
    if (im->bits_per_pixel<32) im->mask=(1<<im->bits_per_pixel)-1; else im->mask=0xFFFFFFFF;
    //text
    im->cursor_x=1; im->cursor_y=1;
    im->top_row=1;
    if (bpp) im->bottom_row=(im->height/im->font); else im->bottom_row=im->height;
    im->bottom_row--; if (im->bottom_row<=0) im->bottom_row=1;
    if (!bpp) return;
    //graphics
    //clipping/scaling
    im->x=((double)im->width)/2.0; im->y=((double)im->height)/2.0;
    im->view_x2=im->width-1; im->view_y2=im->height-1;
    im->scaling_x=1; im->scaling_y=1;
    im->window_x2=im->view_x2; im->window_y2=im->view_y2;
    
    //clear
    if (bpp){//graphics
        memset(im->offset,0,im->width*im->height*im->bytes_per_pixel);
        }else{//text
        static int32 i2,i3;
        static uint16 *sp;
        i3=im->width*im->height; sp=(uint16*)im->offset; for (i2=0;i2<i3;i2++){*sp++=0x0720;}
        qb_init_text_utf8_cells(i);
    }
    
}//imgrevert

int32 imgframe(uint8 *o,int32 x,int32 y,int32 bpp){
    static int32 i;
    static img_struct *im;
    if (x<=0||y<=0) return 0;
    i=newimg();
    im=&img[i];
    qb_set_img_external_offset(i,o);
    im->width=x; im->height=y;
    
    //assume default values
    im->bytes_per_pixel=1;
    im->font=16;
    im->color=15;
    im->compatible_mode=bpp;
    im->print_mode=3;
    im->draw_ta=0.0; im->draw_scale=1.0;
    
    //set values
    switch (bpp){
        case 0:
        im->bits_per_pixel=16; im->bytes_per_pixel=2;
        im->color=7;
        im->text=1;
        im->cursor_show=0; im->cursor_firstvalue=4; im->cursor_lastvalue=4;
        break;
        case 1:
        im->bits_per_pixel=2;
        im->font=8;
        im->color=3;
        break;
        case 2:
        im->bits_per_pixel=1; 
        im->font=8;//it gets stretched from 8 to 16 later
        im->color=1;
        break;
        case 7:
        im->bits_per_pixel=4;
        im->font=8;
        break;
        case 8:
        im->bits_per_pixel=4;
        im->font=8;
        break;
        case 9:
        im->bits_per_pixel=4;
        im->font=14;
        break;
        case 10:
        im->bits_per_pixel=2;
        im->font=14;
        im->color=3;
        break;
        case 11:
        im->bits_per_pixel=1;
        im->color=1;
        break;
        case 12:
        im->bits_per_pixel=4;
        break;
        case 13:
        im->bits_per_pixel=8;
        im->font=8;
        break;
        case 256:
        im->bits_per_pixel=8;
        break;
        case 32:
        im->bits_per_pixel=32; im->bytes_per_pixel=4;
        im->color=0xFFFFFFFF;
        im->background_color=0xFF000000;
        break;
        default:
        return 0;
    };
    im->draw_color=im->color;
    
    //attach palette
    if (bpp!=32){
        qb_allocate_img_palette(i);
        if (!im->pal){
            freeimg(i);
            return 0;
        }
        im->flags|=IMG_FREEPAL;
        restorepalette(im);
        im->transparent_color=-1;
    }
    
    //set calculatable values
    if (im->bits_per_pixel<32) im->mask=(1<<im->bits_per_pixel)-1; else im->mask=0xFFFFFFFF;
    //text
    im->cursor_x=1; im->cursor_y=1;
    im->top_row=1;
    if (bpp) im->bottom_row=(im->height/im->font); else im->bottom_row=im->height;
    im->bottom_row--; if (im->bottom_row<=0) im->bottom_row=1;
    if (!bpp) qb_init_text_utf8_cells(i);
    else qb_release_text_utf8_cells(i);
    if (!bpp) return i;
    //graphics
    //clipping/scaling
    im->x=((double)im->width)/2.0; im->y=((double)im->height)/2.0;
    im->view_x2=im->width-1; im->view_y2=im->height-1;
    im->scaling_x=1; im->scaling_y=1;
    im->window_x2=im->view_x2; im->window_y2=im->view_y2;
    
    return i;
}

void sub__freeimage(int32 i,int32 passed);//forward ref

int32 imgnew(int32 x,int32 y,int32 bpp){
    static int32 i,i2,i3;
    static img_struct *im;
    static uint16 *sp;
    static uint32 *lp;
    i=imgframe(NULL,x,y,bpp);
    if (!i) return 0;
    im=&img[i];
    if (bpp){//graphics
        if (bpp==32){
            if (!cblend) init_blend();
            qb_allocate_img_pixels(i,x*y*4);
            if (!im->offset){sub__freeimage(-i,1); return 0;}
            //i3=x*y; lp=im->offset32; for (i2=0;i2<i3;i2++){*lp++=0xFF000000;}
            }else{
            qb_allocate_img_pixels(i,x*y*im->bytes_per_pixel);
            if (!im->offset){sub__freeimage(-i,1); return 0;}
        }
        }else{//text
        qb_allocate_img_pixels(i,x*y*im->bytes_per_pixel);
        if (!im->offset){sub__freeimage(-i,1); return 0;}
        i3=x*y; sp=(uint16*)im->offset; for (i2=0;i2<i3;i2++){*sp++=0x0720;}
        qb_init_text_utf8_cells(i);
    }
    im->flags|=IMG_FREEMEM;
    return i;
}
