// gamepad_bridge.cpp — _DEVICES/_DEVICEINPUT/STRIG glue for the Rust gamepad
// backend (internal/c/qbrt/src/gamepad.rs, gilrs). Replaces
// internal/c/parts/input/game_controller (libstem gamepad + src.c glue).
// The device bookkeeping below is the legacy src.c logic with the library
// structs replaced by plain arguments.

extern "C" {
    typedef struct qbrt_gamepad_callbacks {
        void (*attached)(uint32_t id, const char *name, int32_t vendor, int32_t product, int32_t axes, int32_t buttons);
        void (*removed)(uint32_t id);
        void (*button)(uint32_t id, int32_t button, int32_t down);
        void (*axis)(uint32_t id, int32_t axis, float value);
    } qbrt_gamepad_callbacks;
    void qbrt_gamepad_poll(const qbrt_gamepad_callbacks *cb);
    void qbrt_gamepad_shutdown(void);
}

static void gamepad_button_down(uint32 deviceID, int32 button){
    int32 di,controller;
    controller=0;
    for(di=1;di<=device_last;di++){
        device_struct *d=&devices[di];
        if (d->used==1&&d->type==1){
            controller++;
            if (d->handle_int==deviceID){
                //ON STRIG event
                if (controller<=256&&button<=255){//within supported range
                    int32 i=(controller-1)*256+button;
                    if (onstrig[i].active){
                        if (onstrig[i].id){
                            if (onstrig[i].active==1){//(1)ON
                                onstrig[i].state++;
                            }else{//(2)STOP
                                onstrig[i].state=1;
                            }
                            qbevent=1;
                        }
                    }
                }
                int32 eventIndex=createDeviceEvent(d);
                setDeviceEventButtonValue(d,eventIndex,button,1);
                commitDeviceEvent(d);
                //set STRIG_button_pressed for button
                if (button>=0&&button<=255) d->STRIG_button_pressed[button]=1;
            }
        }
    }
}

static void gamepad_button_up(uint32 deviceID, int32 button){
    for(int32 di=1;di<=device_last;di++){
        device_struct *d=&devices[di];
        if (d->used==1&&d->type==1&&d->handle_int==deviceID){
            int32 eventIndex=createDeviceEvent(d);
            setDeviceEventButtonValue(d,eventIndex,button,0);
            commitDeviceEvent(d);
        }
    }
}

static void gamepad_axis(uint32 deviceID, int32 axis, float f){
    if (f>1.0) f=1.0;
    if (f<-1.0) f=-1.0;
    for(int32 di=1;di<=device_last;di++){
        device_struct *d=&devices[di];
        if (d->used==1&&d->type==1&&d->handle_int==deviceID){
            int32 eventIndex=createDeviceEvent(d);
            setDeviceEventAxisValue(d,eventIndex,axis,f);
            commitDeviceEvent(d);
        }
    }
}

static void gamepad_attached(uint32 deviceID, const char *description, int32 vendorID, int32 productID, int32 numAxes, int32 numButtons){
    int32 i;
    //re-acquire a potentially dropped device in its original index
    for (i=1;i<=device_last;i++){
        if (devices[i].used&&devices[i].type==1&&!devices[i].connected){
            if (productID==devices[i].product_id&&vendorID==devices[i].vendor_id&&numAxes==devices[i].axes&&numButtons==devices[i].buttons){
                devices[i].handle_int=deviceID;
                devices[i].connected=1;
                devices[i].name[strlen(devices[i].name)-14]=0;//Remove "[DISCONNECTED]"
                devices[i].used=1;
                return;
            }
        }
    }
    //add new device
    i=device_last+1;
    if (i>device_max){
        //deviation D-11: legacy assigned the realloc result to a local (heap overflow)
        devices=(device_struct*)realloc(devices,(device_max*2+1)*sizeof(device_struct));
        device_max*=2;
    }
    memset(&devices[i],0,sizeof(device_struct));
    devices[i].type=DEVICETYPE_CONTROLLER;
    devices[i].description=strdup(description);
    devices[i].handle_int=deviceID;
    devices[i].buttons=numButtons;
    devices[i].lastbutton=devices[i].buttons;
    devices[i].axes=numAxes;
    devices[i].lastaxis=devices[i].axes;
    devices[i].product_id=productID;
    devices[i].vendor_id=vendorID;
    std::string name="[CONTROLLER][[NAME][";
    name+=devices[i].description;
    name+="]]";
    if (devices[i].lastbutton) name+="[BUTTON]";
    if (devices[i].lastaxis) name+="[AXIS]";
    if (devices[i].lastwheel) name+="[WHEEL]";
    devices[i].name=strdup(name.c_str());
    setupDevice(&devices[i]);
    device_last=i;
}

static void gamepad_removed(uint32 deviceID){
    for (int32 i=1;i<=device_last;i++){
        if (devices[i].used&&devices[i].type==1&&devices[i].handle_int==deviceID){
            std::string name=devices[i].name;
            name+="[DISCONNECTED]";
            char *oldname=devices[i].name;
            devices[i].name=strdup(name.c_str());
            free(oldname);
            devices[i].connected=0;
        }
    }
}

extern "C" {
    static void qbrt_gp_attached(uint32_t id,const char *name,int32_t vendor,int32_t product,int32_t axes,int32_t buttons){ gamepad_attached(id,name,vendor,product,axes,buttons); }
    static void qbrt_gp_removed(uint32_t id){ gamepad_removed(id); }
    static void qbrt_gp_button(uint32_t id,int32_t button,int32_t down){ if (down) gamepad_button_down(id,button); else gamepad_button_up(id,button); }
    static void qbrt_gp_axis(uint32_t id,int32_t axis,float value){ gamepad_axis(id,axis,value); }
}

void QBNex_GAMEPAD_INIT(){
    //the gilrs context is created by the first poll, on the MAIN_LOOP thread
}
void QBNex_GAMEPAD_POLL(){
    static qbrt_gamepad_callbacks cb={qbrt_gp_attached,qbrt_gp_removed,qbrt_gp_button,qbrt_gp_axis};
    qbrt_gamepad_poll(&cb);
}
void QBNex_GAMEPAD_SHUTDOWN(){
    qbrt_gamepad_shutdown();
}
