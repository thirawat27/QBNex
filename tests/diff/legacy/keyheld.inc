// VERBATIM copy of keyheld()/keyheld_add/remove/unbind (libqb.cpp 2191-2252) and update_shift_state() (30014-30083) from the legacy libqb.cpp (oracle 545b859). Test-only.
static std::vector<uint32> keyheld_buffer;
static std::vector<uint32> keyheld_bind_buffer;

int32 keyheld(uint32 x){
    for (std::vector<uint32>::const_iterator it=keyheld_buffer.begin(); it!=keyheld_buffer.end(); ++it){
        if (*it==x) return 1;
    }
    //check multimapped NUMPAD keys
    if ((x>=42)&&(x<=57)){
        if ((x>=48)&&(x<=57)) return keyheld(VK+QBVK_KP0+(x-48));//0-9
        if (x==46) return keyheld(VK+QBVK_KP_PERIOD);
        if (x==47) return keyheld(VK+QBVK_KP_DIVIDE);
        if (x==42) return keyheld(VK+QBVK_KP_MULTIPLY);
        if (x==45) return keyheld(VK+QBVK_KP_MINUS);
        if (x==43) return keyheld(VK+QBVK_KP_PLUS);
    }
    if (x==13) return keyheld(VK+QBVK_KP_ENTER);
    if (x&0xFF00){
        static uint32 x2;
        x2=(x>>8)&255;
        if ((x2>=71)&&(x2<=83)){
            if (x2==82) return keyheld(QBK+QBVK_KP0-QBVK_KP0);
            if (x2==79) return keyheld(QBK+QBVK_KP1-QBVK_KP0);
            if (x2==80) return keyheld(QBK+QBVK_KP2-QBVK_KP0);
            if (x2==81) return keyheld(QBK+QBVK_KP3-QBVK_KP0);
            if (x2==75) return keyheld(QBK+QBVK_KP4-QBVK_KP0);
            if (x2==76) return keyheld(QBK+QBVK_KP5-QBVK_KP0);
            if (x2==77) return keyheld(QBK+QBVK_KP6-QBVK_KP0);
            if (x2==71) return keyheld(QBK+QBVK_KP7-QBVK_KP0);
            if (x2==72) return keyheld(QBK+QBVK_KP8-QBVK_KP0);
            if (x2==73) return keyheld(QBK+QBVK_KP9-QBVK_KP0);
            if (x2==83) return keyheld(QBK+QBVK_KP_PERIOD-QBVK_KP0);
        }
    }
    return 0;
}


void keyheld_add(uint32 x){
    if (std::find(keyheld_buffer.begin(), keyheld_buffer.end(), x)!=keyheld_buffer.end()) return;//already in buffer
    keyheld_buffer.push_back(x);//add entry
    keyheld_bind_buffer.push_back(bindkey);//add binded key (0=none)
    bindkey=0;
}
void keyheld_remove(uint32 x){
    for (std::vector<uint32>::size_type i=0; i<keyheld_buffer.size(); ++i){
        if (keyheld_buffer[i]==x){//exists
            keyheld_buffer.erase(keyheld_buffer.begin()+static_cast<std::vector<uint32>::difference_type>(i));
            keyheld_bind_buffer.erase(keyheld_bind_buffer.begin()+static_cast<std::vector<uint32>::difference_type>(i));
            return;
        }
    }
}
void keyheld_unbind(uint32 x){
    for (std::vector<uint32>::size_type i=0; i<keyheld_bind_buffer.size(); ++i){
        if (keyheld_bind_buffer[i]==x){//exists
            keyup(keyheld_buffer[i]);
            return;
        }
    }
}

void update_shift_state(){
    int32 x;
    /*
        0:417h                   Shift Status
        7 6 5 4 3 2 1 0
        x . . . . . . .      Insert locked
        . x . . . . . .      Caps Lock locked
        . . x . . . . .      Num Lock locked
        . . . x . . . .      Scroll Lock locked
        . . . . x . . .      Alt key is pressed
        . . . . . x . .      Ctrl key is pressed
        . . . . . . x .      Left Shift key is pressed
        . . . . . . . x      Right Shift key is pressed
    */
    x=0;
    if (keyheld(VK+QBVK_RSHIFT)) x|=1;
    if (keyheld(VK+QBVK_LSHIFT)) x|=2;
    if (keyheld(VK+QBVK_LCTRL)||keyheld(VK+QBVK_RCTRL)) x|=4;
    if (keyheld(VK+QBVK_LALT)||keyheld(VK+QBVK_RALT)) x|=8;
    if (keyheld(QBK+QBK_SCROLL_LOCK_MODE)) x|=16;
    if (keyheld(VK+QBVK_NUMLOCK)) x|=32;
    if (keyheld(VK+QBVK_CAPSLOCK)) x|=64;
    //note: insert state is emulated (off by default)
    if (keyheld(QBK+QBK_INSERT_MODE)) x|=128;
    cmem[0x417]=x;
    /*
        0:418h                   Extended Shift Status
        (interpret the word 'pressed' as "being held down")
        7 6 5 4 3 2 1 0
        x . . . . . . .      Ins key is pressed
        . x . . . . . .      Caps Lock key is pressed (detection not possible, return 0)
        . . x . . . . .      Num Lock key is pressed (detection not possible, return 0)
        . . . x . . . .      Scroll Lock key is pressed
        . . . . x . . .      Pause key locked
        . . . . . x . .      SysReq key is pressed
        . . . . . . x .      Left Alt key is pressed
        . . . . . . . x      Left Ctrl key is pressed
    */
    x=0;
    if (keyheld(VK+QBVK_LCTRL)) x|=1;
    if (keyheld(VK+QBVK_LALT)) x|=2;
    if (keyheld(VK+QBVK_SYSREQ)) x|=4;
    if (keyheld(VK+QBVK_PAUSE)) x|=8;
    if (keyheld(VK+QBVK_SCROLLOCK)) x|=16;
    //if (keyheld(VK+QBVK_NUMLOCK)) x|=32;
    //if (keyheld(VK+QBVK_CAPSLOCK)) x|=64;
    if (keyheld(0x5200)) x|=128;
    cmem[0x418]=x;
    /*
        0:496h                   Keyboard Status and Type Flags
        This byte holds keyboard status information.
        Keyboard Status Information
        7 6 5 4 3 2 1 0
        x . . . . . . .       Read ID in progress (always 0)
        . x . . . . . .       Last character was first ID character (always 0)
        . . x . . . . .       Force Num Lock if read ID and KBX (always 0)
        . . . x . . . .       101/102-key keyboard installed (always 1)
        . . . . x . . .       Right Alt key is pressed
        . . . . . x . .       Right Ctrl key is pressed
        . . . . . . x .       Last code was E0 Hidden Code (always 0)
        . . . . . . . x       last code was E1 Hidden Code (always 0)
    */
    x=0;
    if (keyheld(VK+QBVK_RCTRL)) x|=1;
    if (keyheld(VK+QBVK_RALT)) x|=2;
    x|=16;
    cmem[0x496]=x;
}


