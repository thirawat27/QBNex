// window_bridge.cpp — glue between libqb.cpp and the Rust window layer
// (crates/qb-runtime/src/window.rs, raster.rs, keybuf.rs). Included by libqb.cpp
// in place of the old libqb/gui.cpp (freeglut + OpenGL presenter).
//
// Composition model: the thread that builds a display frame (MAIN_LOOP, or
// the program thread on _DISPLAY) also composes it here — software SCREEN
// plus "hardware" image commands, rendered in software by qb_hw_* — and
// hands the result to qb_window_present(). The window thread only scales it.
// display_frame[] and the hardware command list are therefore touched by one
// thread at a time (present_mutex), which removes the GLUT-thread races (F3).

int32 displayorder_screen=1;
int32 displayorder_hardware=2;
int32 displayorder_glrender=3;
int32 displayorder_hardware1=4;

void sub__displayorder(int32 method1,int32 method2,int32 method3,int32 method4){
    //check no value has been used twice
    if (method1!=0) if (method1==method2||method1==method3||method1==method4){error(5); return;}
    if (method2!=0) if (method2==method1||method2==method3||method2==method4){error(5); return;}
    if (method3!=0) if (method3==method1||method3==method2||method3==method4){error(5); return;}
    if (method4!=0) if (method4==method1||method4==method2||method4==method3){error(5); return;}
    displayorder_screen=0;
    displayorder_hardware=0;
    displayorder_hardware1=0;
    displayorder_glrender=0;
    static int32 i,method;
    for (i=1;i<=4;i++){
        if (i==1) method=method1;
        if (i==2) method=method2;
        if (i==3) method=method3;
        if (i==4) method=method4;
        if (method==1) displayorder_screen=i;
        if (method==2) displayorder_hardware=i;
        if (method==3) displayorder_hardware1=i;
        if (method==4) displayorder_glrender=i;
    }
}

void sub__glrender(int32 method){
    if (method==1) sub__displayorder(4,1,2,3);
    if (method==2) sub__displayorder(1,2,4,3);
    if (method==3) sub__displayorder(4,0,0,0);
}

// ---- hardware images (kept as software pixel buffers) -------------------------

static std::unordered_map<int32,std::vector<float> > hardware_img_depth;

int32 new_hardware_img(int32 x, int32 y, uint32 *pixels, int32 flags){
    int32 handle;
    hardware_img_struct* hardware_img;
    handle=list_add(hardware_img_handles);
    hardware_img=(hardware_img_struct*)list_get(hardware_img_handles,handle);
    hardware_img->w=x;
    hardware_img->h=y;
    hardware_img->dest_context_handle=0;
    hardware_img->depthbuffer_handle=0;
    hardware_img->pending_commands=0;
    hardware_img->remove=0;
    hardware_img->alpha_disabled=0;
    hardware_img->depthbuffer_mode=DEPTHBUFFER_MODE__ON;
    hardware_img->valid=1;
    hardware_img->source_state.PO2_fix=PO2_FIX__OFF;
    hardware_img->source_state.texture_wrap=TEXTURE_WRAP_MODE__UNKNOWN;
    hardware_img->source_state.smooth_stretched=SMOOTH_MODE__UNKNOWN;
    hardware_img->source_state.smooth_shrunk=SMOOTH_MODE__UNKNOWN;
    hardware_img->texture_handle=0;
    #ifdef QBNex_GUI
        if (flags&NEW_HARDWARE_IMG__BUFFER_CONTENT){
            if (flags&NEW_HARDWARE_IMG__DUPLICATE_PROVIDED_BUFFER){
                hardware_img->software_pixel_buffer=(uint32*)malloc(x*y*4);
                memcpy(hardware_img->software_pixel_buffer,pixels,x*y*4);
            }else{
                hardware_img->software_pixel_buffer=pixels;//consumed
            }
        }else{
            hardware_img->software_pixel_buffer=(uint32*)calloc((size_t)x*(size_t)y,4);//blank surface
        }
    #else
        //console only: nothing is ever displayed
        if ((flags&NEW_HARDWARE_IMG__BUFFER_CONTENT)&&!(flags&NEW_HARDWARE_IMG__DUPLICATE_PROVIDED_BUFFER)) free(pixels);
        hardware_img->software_pixel_buffer=NULL;
    #endif
    return handle;
}

void free_hardware_img(int32 handle, int32 caller_id){
    hardware_img_struct* hardware_img=(hardware_img_struct*)list_get(hardware_img_handles,handle);
    if (hardware_img==NULL) return;
    if (hardware_img->software_pixel_buffer!=NULL) free(hardware_img->software_pixel_buffer);
    hardware_img->software_pixel_buffer=NULL;
    hardware_img_depth.erase(handle);
    list_remove(hardware_img_handles,handle);
}

#ifdef QBNex_GUI

//present_mutex: declared in libqb.cpp (also used by display())
static std::vector<uint32> present_buffer;   //composed SCREEN-sized frame
static std::vector<float> present_depth;     //depth buffer of the composed frame
static int32 present_w=0,present_h=0;

struct hw_surface{ uint32 *px; int32 w; int32 h; float *depth; };

static int32 hw_surface_get(int32 dst_img,hw_surface *s,int32 need_depth){
    if (dst_img<=0){//both display layers render to the composed frame
        s->px=present_buffer.data(); s->w=present_w; s->h=present_h; s->depth=present_depth.data();
        return present_w>0&&present_h>0;
    }
    hardware_img_struct *d=(hardware_img_struct*)list_get(hardware_img_handles,dst_img);
    if (d==NULL||d->software_pixel_buffer==NULL) return 0;
    s->px=d->software_pixel_buffer; s->w=d->w; s->h=d->h; s->depth=NULL;
    if (need_depth){
        std::vector<float> &z=hardware_img_depth[dst_img];
        if (z.size()!=(size_t)d->w*(size_t)d->h) z.assign((size_t)d->w*(size_t)d->h,INFINITY);
        s->depth=z.data();
    }
    return 1;
}

static hardware_img_struct *hw_source(int32 src_img){
    hardware_img_struct *s=(hardware_img_struct*)list_get(hardware_img_handles,src_img);
    if (s==NULL||s->software_pixel_buffer==NULL) return NULL;
    return s;
}

void hardware_img_put(int32 dst_x1,int32 dst_y1,int32 dst_x2,int32 dst_y2,int32 src_img,int32 dst_img,
int32 src_x1,int32 src_y1,int32 src_x2,int32 src_y2,int32 use_alpha,int32 smooth){
    hw_surface d; if (!hw_surface_get(dst_img,&d,0)) return;
    hardware_img_struct *s=hw_source(src_img); if (!s) return;
    int32 dr[4]={dst_x1,dst_y1,dst_x2,dst_y2};
    int32 sr[4]={src_x1,src_y1,src_x2,src_y2};
    qb_hw_put(d.px,d.w,d.h,s->software_pixel_buffer,s->w,s->h,dr,sr,use_alpha,smooth);
}

void hardware_img_tri2d(float dst_x1,float dst_y1,float dst_x2,float dst_y2,float dst_x3,float dst_y3,
int32 src_img,int32 dst_img,float src_x1,float src_y1,float src_x2,float src_y2,float src_x3,float src_y3,
int32 use_alpha,int32 smooth){
    hw_surface d; if (!hw_surface_get(dst_img,&d,0)) return;
    hardware_img_struct *s=hw_source(src_img); if (!s) return;
    float dv[6]={dst_x1,dst_y1,dst_x2,dst_y2,dst_x3,dst_y3};
    float sv[6]={src_x1,src_y1,src_x2,src_y2,src_x3,src_y3};
    qb_hw_tri2d(d.px,d.w,d.h,s->software_pixel_buffer,s->w,s->h,dv,sv,use_alpha,smooth);
}

void hardware_img_tri3d(float dst_x1,float dst_y1,float dst_z1,float dst_x2,float dst_y2,float dst_z2,float dst_x3,float dst_y3,float dst_z3,
int32 src_img,int32 dst_img,float src_x1,float src_y1,float src_x2,float src_y2,float src_x3,float src_y3,
int32 use_alpha,int32 smooth,int32 cull_mode,int32 depthbuffer_mode){
    hw_surface d; if (!hw_surface_get(dst_img,&d,1)) return;
    hardware_img_struct *s=hw_source(src_img); if (!s) return;
    float dv[9]={dst_x1,dst_y1,dst_z1,dst_x2,dst_y2,dst_z2,dst_x3,dst_y3,dst_z3};
    float sv[6]={src_x1,src_y1,src_x2,src_y2,src_x3,src_y3};
    qb_hw_tri3d(d.px,d.w,d.h,d.depth,s->software_pixel_buffer,s->w,s->h,dv,sv,use_alpha,smooth,cull_mode,depthbuffer_mode);
}

void clear_depthbuffer(int32 dst_img){
    hw_surface d; if (!hw_surface_get(dst_img,&d,1)) return;
    if (d.depth) std::fill(d.depth,d.depth+(size_t)d.w*(size_t)d.h,INFINITY);
}

static void present_update_environment(){
    int32 l[8]; float sc[2];
    qb_layout(environment_2d__screen_width,environment_2d__screen_height,environment__window_width,environment__window_height,
    full_screen,fullscreen_smooth,resize_auto,l,sc);
    environment_2d__screen_x1=l[0]; environment_2d__screen_y1=l[1];
    environment_2d__screen_x2=l[2]; environment_2d__screen_y2=l[3];
    environment_2d__screen_scaled_width=l[4]; environment_2d__screen_scaled_height=l[5];
    environment_2d__screen_smooth=l[6]; environment_2d__letterbox=l[7];
    environment_2d__screen_x_scale=sc[0]; environment_2d__screen_y_scale=sc[1];
}

//Port of GLUT_DISPLAY_REQUEST (gui.cpp) without OpenGL. Called by the thread that
//built the display frame, right after display(), and by MAIN_LOOP after resizes.
void present_frame(){
    std::lock_guard<std::recursive_mutex> guard(present_mutex);
    static int32 i,i2;
    //determine which software frame to display
    static int32 last_i;//the last software frame displayed
    last_i=-1;
    for (i2=0;i2<=2;i2++){
        if (display_frame[i2].state==DISPLAY_FRAME_STATE__DISPLAYING) last_i=i2;
    }
    i=-1;
    static int64 highest_order;
    highest_order=0;
    if (last_i!=-1) highest_order=display_frame[last_i].order;//avoid any frames below the current one
    for (i2=0;i2<=2;i2++){
        if (display_frame[i2].state==DISPLAY_FRAME_STATE__READY&&display_frame[i2].order>highest_order){
            highest_order=display_frame[i2].order;
            i=i2;
        }
    }
    if (i==-1) i=last_i;
    if (i==-1) return;//no frames exist yet
    if (i!=last_i){
        for (i2=0;i2<=2;i2++){
            if (display_frame[i2].order<display_frame[i].order&&(display_frame[i2].state==DISPLAY_FRAME_STATE__DISPLAYING||display_frame[i2].state==DISPLAY_FRAME_STATE__READY)) display_frame[i2].state=DISPLAY_FRAME_STATE__EMPTY;
        }
        display_frame[i].state=DISPLAY_FRAME_STATE__DISPLAYING;
    }
    static int64 order;
    order=last_hardware_display_frame_order;
    if (last_rendered_hardware_display_frame_order==last_hardware_display_frame_order){
        if (i==last_i&&full_screen_set==-1&&os_resize_event==0) return;//nothing new
    }
    environment_2d__screen_width=display_frame[i].w;
    environment_2d__screen_height=display_frame[i].h;
    os_resize_event=0;
    if ((full_screen==0)&&(full_screen_set==-1)){//not in (or attempting to enter) full screen
        display_required_x=display_frame[i].w; display_required_y=display_frame[i].h;
        static int32 framesize_changed;
        framesize_changed=0;
        if ((display_required_x!=resize_snapback_x)||(display_required_y!=resize_snapback_y)) framesize_changed=1;
        resize_auto_ideal_aspect=(float)display_frame[i].w/(float)display_frame[i].h;
        resize_snapback_x=display_required_x; resize_snapback_y=display_required_y;
        if (resize_auto){
            //maintain aspect ratio
            static float ar;
            ar=(float)display_x/(float)display_y;
            if ((ar!=resize_auto_accept_aspect)&&(ar!=resize_auto_ideal_aspect)){
                static int32 x,y;
                if (display_x_prev==display_x){
                    y=display_y;
                    x=(float)y*resize_auto_ideal_aspect;
                }
                if (display_y_prev==display_y){
                    x=display_x;
                    y=(float)x/resize_auto_ideal_aspect;
                }
                if ((display_y_prev!=display_y)&&(display_x_prev!=display_x)){
                    if (abs(display_y_prev-display_y)<abs(display_x_prev-display_x)){
                        x=display_x;
                        y=(float)x/resize_auto_ideal_aspect;
                    }else{
                        y=display_y;
                        x=(float)y*resize_auto_ideal_aspect;
                    }
                }
                resize_auto_accept_aspect=(float)x/(float)y;
                resize_pending=1;
                qb_window_set_size(x,y);
                goto auto_resized;
            }
        }//resize_auto
        if ((display_required_x!=display_x)||(display_required_y!=display_y)){
            if (resize_snapback||framesize_changed){
                qb_window_set_size(display_required_x,display_required_y);
                resize_pending=1;
            }
        }
        auto_resized:;
    }
    //Pseudo-Fullscreen
    if (!resize_pending){//avoid switching to fullscreen before resize operations take effect
        if (full_screen_set!=-1){//full screen mode change requested
            if (full_screen_set==0){
                if (full_screen!=0){
                    //exit full screen
                    resize_pending=1;
                    qb_window_set_fullscreen(0);
                    qb_window_set_size(display_frame[i].w,display_frame[i].h);
                }
                full_screen=0;
                full_screen_set=-1;
            }else{
                if (full_screen==0) qb_window_set_fullscreen(1);
                full_screen=full_screen_set;
                full_screen_set=-1;
            }
        }
    }
    environment__window_width=display_x;
    environment__window_height=display_y;
    present_update_environment();
    //compose
    present_w=display_frame[i].w; present_h=display_frame[i].h;
    present_buffer.assign((size_t)present_w*(size_t)present_h,0);
    present_depth.assign((size_t)present_w*(size_t)present_h,INFINITY);
    static int32 first_hardware_layer_rendered;
    static int32 first_hardware_layer_command;
    first_hardware_layer_rendered=0;
    first_hardware_layer_command=0;
    static int32 level;
    for (level=1;level<=4;level++){
        if (level==displayorder_screen){
            int32 use_alpha=0; if (level>1) use_alpha=1;
            if (!use_alpha){
                memcpy(present_buffer.data(),display_frame[i].bgra,(size_t)present_w*(size_t)present_h*4);
            }else{
                int32 dr[4]={0,0,present_w-1,present_h-1};
                qb_hw_put(present_buffer.data(),present_w,present_h,display_frame[i].bgra,present_w,present_h,dr,dr,1,0);
            }
        }
        if (level==displayorder_hardware||level==displayorder_hardware1){
            static int32 dst;
            dst=0; if (level==displayorder_hardware1) dst=-1;
            static int32 command;
            command=0;
            if (first_hardware_layer_rendered==0){
                if (first_hardware_command){
                    if (last_hardware_command_rendered){
                        hardware_graphics_command_struct* last_hgc=(hardware_graphics_command_struct*)list_get(hardware_graphics_command_handles,last_hardware_command_rendered);
                        command=last_hgc?last_hgc->next_command:0;
                    }else{
                        command=first_hardware_command;
                    }
                    //process/skip pending hardware commands before this frame's order value
                    while (command){
                        hardware_graphics_command_struct* hgc=(hardware_graphics_command_struct*)list_get(hardware_graphics_command_handles,command);
                        if (hgc==NULL) break;
                        if (hgc->order<order){
                            if (hgc->command==HARDWARE_GRAPHICS_COMMAND__FREEIMAGE) free_hardware_img(hgc->src_img,847002);
                            if (hgc->dst_img>0){//renders onto maintained hardware images are still required
                                if (hgc->command==HARDWARE_GRAPHICS_COMMAND__PUTIMAGE) hardware_img_put(hgc->dst_x1,hgc->dst_y1,hgc->dst_x2,hgc->dst_y2,hgc->src_img,hgc->dst_img,hgc->src_x1,hgc->src_y1,hgc->src_x2,hgc->src_y2,hgc->use_alpha,hgc->smooth);
                                if (hgc->command==HARDWARE_GRAPHICS_COMMAND__MAPTRIANGLE) hardware_img_tri2d(hgc->dst_x1,hgc->dst_y1,hgc->dst_x2,hgc->dst_y2,hgc->dst_x3,hgc->dst_y3,hgc->src_img,hgc->dst_img,hgc->src_x1,hgc->src_y1,hgc->src_x2,hgc->src_y2,hgc->src_x3,hgc->src_y3,hgc->use_alpha,hgc->smooth);
                                if (hgc->command==HARDWARE_GRAPHICS_COMMAND__MAPTRIANGLE3D) hardware_img_tri3d(hgc->dst_x1,hgc->dst_y1,hgc->dst_z1,hgc->dst_x2,hgc->dst_y2,hgc->dst_z2,hgc->dst_x3,hgc->dst_y3,hgc->dst_z3,hgc->src_img,hgc->dst_img,hgc->src_x1,hgc->src_y1,hgc->src_x2,hgc->src_y2,hgc->src_x3,hgc->src_y3,hgc->use_alpha,hgc->smooth,hgc->cull_mode,hgc->depthbuffer_mode);
                                if (hgc->command==HARDWARE_GRAPHICS_COMMAND__CLEAR_DEPTHBUFFER) clear_depthbuffer(hgc->dst_img);
                            }
                            last_hardware_command_rendered=command;
                            if (next_hardware_command_to_remove==0) next_hardware_command_to_remove=command;
                            command=hgc->next_command;
                            hgc->remove=1;
                        }else{
                            break;
                        }
                    }
                }
                first_hardware_layer_command=command;
            }else{
                command=first_hardware_layer_command;
            }
            //process pending hardware commands for this frame's order value
            while (command){
                hardware_graphics_command_struct* hgc=(hardware_graphics_command_struct*)list_get(hardware_graphics_command_handles,command);
                if (hgc==NULL) break;
                if (hgc->order!=order) break;
                if (hgc->command==HARDWARE_GRAPHICS_COMMAND__FREEIMAGE&&first_hardware_layer_rendered==0) free_hardware_img(hgc->src_img,847003);
                if ((hgc->dst_img>0&&first_hardware_layer_rendered==0)||hgc->dst_img==dst){
                    if (hgc->command==HARDWARE_GRAPHICS_COMMAND__PUTIMAGE) hardware_img_put(hgc->dst_x1,hgc->dst_y1,hgc->dst_x2,hgc->dst_y2,hgc->src_img,hgc->dst_img,hgc->src_x1,hgc->src_y1,hgc->src_x2,hgc->src_y2,hgc->use_alpha,hgc->smooth);
                    if (hgc->command==HARDWARE_GRAPHICS_COMMAND__MAPTRIANGLE) hardware_img_tri2d(hgc->dst_x1,hgc->dst_y1,hgc->dst_x2,hgc->dst_y2,hgc->dst_x3,hgc->dst_y3,hgc->src_img,hgc->dst_img,hgc->src_x1,hgc->src_y1,hgc->src_x2,hgc->src_y2,hgc->src_x3,hgc->src_y3,hgc->use_alpha,hgc->smooth);
                    if (hgc->command==HARDWARE_GRAPHICS_COMMAND__MAPTRIANGLE3D) hardware_img_tri3d(hgc->dst_x1,hgc->dst_y1,hgc->dst_z1,hgc->dst_x2,hgc->dst_y2,hgc->dst_z2,hgc->dst_x3,hgc->dst_y3,hgc->dst_z3,hgc->src_img,hgc->dst_img,hgc->src_x1,hgc->src_y1,hgc->src_x2,hgc->src_y2,hgc->src_x3,hgc->src_y3,hgc->use_alpha,hgc->smooth,hgc->cull_mode,hgc->depthbuffer_mode);
                    if (hgc->command==HARDWARE_GRAPHICS_COMMAND__CLEAR_DEPTHBUFFER) clear_depthbuffer(hgc->dst_img);
                }
                last_hardware_command_rendered=command;
                if (next_hardware_command_to_remove==0) next_hardware_command_to_remove=command;
                command=hgc->next_command;
                hgc->remove=1;
            }
            first_hardware_layer_rendered=1;
        }
    }//level
    last_rendered_hardware_display_frame_order=last_hardware_display_frame_order;
    qb_3d_present(present_buffer.data(),present_w,present_h);//R15 G4: shown _DRAW3D layer over the frame
    if (!suspend_program) qb_window_present(present_buffer.data(),present_w,present_h,full_screen,fullscreen_smooth,resize_auto);
}

// ---- OS event callbacks (window thread or input script thread) ------------------

static mouse_message_queue_struct *mouse_queue_lock_first(){
    return (mouse_message_queue_struct*)list_get(mouse_message_queue_handles,mouse_message_queue_first);
}

static int32 mouse_queue_next(mouse_message_queue_struct *queue){
    int32 i=queue->last+1; if (i>queue->lastIndex) i=0;
    if (i==queue->current){
        int32 nextIndex=queue->last+1; if (nextIndex>queue->lastIndex) nextIndex=0;
        queue->current=nextIndex;
    }
    return i;
}

static void mouse_button_up(int button,int x,int y){
    mouse_message_queue_struct *queue=mouse_queue_lock_first();
    int32 i=mouse_queue_next(queue);
    queue->queue[i].x=x;
    queue->queue[i].y=y;
    queue->queue[i].movementx=0;
    queue->queue[i].movementy=0;
    queue->queue[i].buttons=queue->queue[queue->last].buttons;
    if (queue->queue[i].buttons&(1<<(button-1))) queue->queue[i].buttons^=(1<<(button-1));
    queue->last=i;
    if (device_last){//core devices required?
        if ((button>=1)&&(button<=3)){
            button--;
            device_struct *d=&devices[2];//mouse
            int32 eventIndex=createDeviceEvent(d);
            setDeviceEventButtonValue(d,eventIndex,button,0);
            commitDeviceEvent(d);
        }
    }
}

static void mouse_button_down(int button,int x,int y){
    mouse_message_queue_struct *queue=mouse_queue_lock_first();
    int32 i=mouse_queue_next(queue);
    queue->queue[i].x=x;
    queue->queue[i].y=y;
    queue->queue[i].movementx=0;
    queue->queue[i].movementy=0;
    queue->queue[i].buttons=queue->queue[queue->last].buttons;
    queue->queue[i].buttons|=(1<<(button-1));
    queue->last=i;
    if (device_last){//core devices required?
        if ((button>=1)&&(button<=3)){
            button--;
            device_struct *d=&devices[2];//mouse
            int32 eventIndex=createDeviceEvent(d);
            setDeviceEventButtonValue(d,eventIndex,button,1);
            commitDeviceEvent(d);
        }else{
            if ((button>=4)&&(button<=5)){
                float f;
                if (button==4) f=-1; else f=1;
                device_struct *d=&devices[2];//mouse
                int32 eventIndex=createDeviceEvent(d);
                setDeviceEventWheelValue(d,eventIndex,2,f);
                commitDeviceEvent(d);
                eventIndex=createDeviceEvent(d);
                setDeviceEventWheelValue(d,eventIndex,2,0);
                commitDeviceEvent(d);
            }
        }
    }
}

static void mouse_motion(int x,int y){
    int32 i,last_i;
    int32 xrel,yrel;
    mouse_message_queue_struct *queue=mouse_queue_lock_first();
    last_i=queue->last;
    i=mouse_queue_next(queue);
    #ifdef QBNex_WINDOWS
        xrel=0; yrel=0;//relative movement comes from raw mouse input
    #else
        xrel=x-queue->queue[queue->last].x;
        yrel=y-queue->queue[queue->last].y;
    #endif
    queue->queue[i].x=x;
    queue->queue[i].y=y;
    queue->queue[i].movementx=xrel;
    queue->queue[i].movementy=yrel;
    queue->queue[i].buttons=queue->queue[last_i].buttons;
    queue->last=i;
    last_i=queue->last;
    i=mouse_queue_next(queue);
    queue->queue[i].x=x;
    queue->queue[i].y=y;
    queue->queue[i].movementx=0;
    queue->queue[i].movementy=0;
    queue->queue[i].buttons=queue->queue[last_i].buttons;
    queue->last=i;
    if (device_last){//core devices required?
        device_struct *d=&devices[2];//mouse
        if (!device_mouse_relative){
            int32 eventIndex=createDeviceEvent(d);
            float fx,fy;
            int32 z;
            fx=x;
            fx-=x_offset;
            z=x_monitor-x_offset*2;
            if (fx<0) fx=0;
            if (fx>=z) fx=z-1;
            fx=fx/(float)(z-1);//0 to 1
            fx*=2.0;//0 to 2
            fx-=1.0;//-1 to 1
            fy=y;
            fy-=y_offset;
            z=y_monitor-y_offset*2;
            if (fy<0) fy=0;
            if (fy>=z) fy=z-1;
            fy=fy/(float)(z-1);//0 to 1
            fy*=2.0;//0 to 2
            fy-=1.0;//-1 to 1
            setDeviceEventAxisValue(d,eventIndex,0,fx);
            setDeviceEventAxisValue(d,eventIndex,1,fy);
            commitDeviceEvent(d);
        }else{
            int32 eventIndex=createDeviceEvent(d);
            setDeviceEventWheelValue(d,eventIndex,0,xrel);
            setDeviceEventWheelValue(d,eventIndex,1,yrel);
            commitDeviceEvent(d);
            eventIndex=createDeviceEvent(d);
            setDeviceEventWheelValue(d,eventIndex,0,0);
            setDeviceEventWheelValue(d,eventIndex,1,0);
            commitDeviceEvent(d);
        }
    }
}

static void mouse_relative(int dx,int dy){
    //was QBNex_EVENT_RELATIVE_MOUSE_MOVEMENT (raw input in freeglut, Windows only)
    #ifdef QBNex_WINDOWS
        mouse_message_queue_struct *queue=mouse_queue_lock_first();
        int32 i=mouse_queue_next(queue);
        queue->queue[i].x=queue->queue[queue->last].x;
        queue->queue[i].y=queue->queue[queue->last].y;
        queue->queue[i].movementx=dx;
        queue->queue[i].movementy=dy;
        queue->queue[i].buttons=queue->queue[queue->last].buttons;
        queue->last=i;
        i=mouse_queue_next(queue);
        queue->queue[i].x=queue->queue[queue->last].x;
        queue->queue[i].y=queue->queue[queue->last].y;
        queue->queue[i].movementx=0;
        queue->queue[i].movementy=0;
        queue->queue[i].buttons=queue->queue[queue->last].buttons;
        queue->last=i;
    #endif
}

extern "C" {
    static void qb_cb_key(int32_t action,uint32_t code){
        qb_input_lock();
        if (action==1) keydown(code);
        if (action==2) keyup(code);
        if (action==3) keydown_unicode(code);
        if (action==4) keyup_unicode(code);
        qb_input_unlock();
    }
    static void qb_cb_mouse_button(int32_t glut_button,int32_t down,int32_t x,int32_t y){
        qb_input_lock();
        if (down) mouse_button_down(glut_button+1,x,y); else mouse_button_up(glut_button+1,x,y);
        qb_input_unlock();
    }
    static void qb_cb_mouse_move(int32_t x,int32_t y){
        qb_input_lock();
        mouse_motion(x,y);
        qb_input_unlock();
    }
    static void qb_cb_mouse_wheel(int32_t direction,int32_t x,int32_t y){
        qb_input_lock();
        if (direction>0){mouse_button_down(4,x,y); mouse_button_up(4,x,y);}
        if (direction<0){mouse_button_down(5,x,y); mouse_button_up(5,x,y);}
        qb_input_unlock();
    }
    static void qb_cb_mouse_relative(int32_t dx,int32_t dy){
        qb_input_lock();
        mouse_relative(dx,dy);
        qb_input_unlock();
    }
    static void qb_cb_resize(int32_t width,int32_t height){
        //was GLUT_RESHAPE_FUNC
        resize_event_x=width; resize_event_y=height;
        resize_event=-1;
        display_x_prev=display_x,display_y_prev=display_y;
        display_x=width; display_y=height;
        resize_pending=0;
        os_resize_event=1;
    }
    static void qb_cb_close(){
        exit_value|=1;
    }
    static void qb_cb_focus(int32_t focused){
        window_focused=focused?-1:0;
    }
    static void qb_cb_file_drop(const char *path){
        if (acceptFileDrop) dropped_files.push_back(std::string(path));
    }
    static void qb_cb_created(void *handle){
        QBNex_Window_Handle(handle);
        window_exists=1;
    }
}

//Runs the window system on the main thread (never returns in windowed mode).
void window_main(){
    static qb_window_callbacks cb;
    cb.key=qb_cb_key;
    cb.mouse_button=qb_cb_mouse_button;
    cb.mouse_move=qb_cb_mouse_move;
    cb.mouse_wheel=qb_cb_mouse_wheel;
    cb.mouse_relative=qb_cb_mouse_relative;
    cb.resize=qb_cb_resize;
    cb.close=qb_cb_close;
    cb.focus=qb_cb_focus;
    cb.file_drop=qb_cb_file_drop;
    cb.created=qb_cb_created;
    qb_window_run(&cb,640,400,window_title?(char*)window_title:"Untitled",ScreenResize,screen_hide?1:0);
}

#else //console only

void present_frame(){}
void window_main(){}

#endif //QBNex_GUI
