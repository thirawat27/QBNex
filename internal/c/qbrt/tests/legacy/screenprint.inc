// Verbatim copy of legacy libqb.cpp (commit 545b859) lines 25039-25917: sub__screenclick, MACVK tables, sub__screenprint. Test-only.
        void sub__screenclick(int32 x,int32 y, int32 button, int32 passed){

            #ifdef QBNex_WINDOWS
                
                static INPUT input;
                
                ZeroMemory(&input,sizeof(INPUT));
                input.type=INPUT_MOUSE;
                input.mi.dwFlags=MOUSEEVENTF_ABSOLUTE|MOUSEEVENTF_MOVE;
                static HWND hwnd;
                hwnd=GetDesktopWindow();
                static RECT rect;
                GetWindowRect(hwnd,&rect);
                static double x2,y2,fx,fy;
                x2=rect.right-rect.left;
                y2=rect.bottom-rect.top;
                fx=x*(65535.0f/x2);
                fy=y*(65535.0f/y2);
                input.mi.dx=fx;
                input.mi.dy=fy;
                SendInput(1,&input,sizeof(INPUT));
                
                ZeroMemory(&input,sizeof(INPUT));
                input.type=INPUT_MOUSE;
                input.mi.dwFlags=MOUSEEVENTF_LEFTDOWN;
                SendInput(1,&input,sizeof(INPUT));
                
                ZeroMemory(&input,sizeof(INPUT));
                input.type=INPUT_MOUSE;
                
                if (passed){
                    if (button==1) {input.mi.dwFlags=MOUSEEVENTF_LEFTDOWN;}
                    if (button==2) {input.mi.dwFlags=MOUSEEVENTF_RIGHTDOWN;}
                    if (button==3) {input.mi.dwFlags=MOUSEEVENTF_MIDDLEDOWN;}
                    SendInput(1,&input,sizeof(INPUT));
                    
                    ZeroMemory(&input,sizeof(INPUT));
                    input.type=INPUT_MOUSE;
                    
                    if (button==1) {input.mi.dwFlags=MOUSEEVENTF_LEFTUP;}
                    if (button==2) {input.mi.dwFlags=MOUSEEVENTF_RIGHTUP;}
                    if (button==3) {input.mi.dwFlags=MOUSEEVENTF_MIDDLEUP;}
                    }else {
                    input.mi.dwFlags=MOUSEEVENTF_LEFTDOWN;
                    
                    SendInput(1,&input,sizeof(INPUT));
                    
                    ZeroMemory(&input,sizeof(INPUT));
                    input.type=INPUT_MOUSE;
                    
                    input.mi.dwFlags=MOUSEEVENTF_LEFTUP;
                }
                SendInput(1,&input,sizeof(INPUT));
                
            #endif
            
            #ifdef QBNex_MACOSX
                CGEventRef click1_down = CGEventCreateMouseEvent(
                NULL, kCGEventLeftMouseDown,
                CGPointMake(x, y),
                kCGMouseButtonLeft
                );
                CGEventRef click1_up = CGEventCreateMouseEvent(
                NULL, kCGEventLeftMouseUp,
                CGPointMake(x, y),
                kCGMouseButtonLeft
                );
                CGEventPost(kCGHIDEventTap, click1_down);
                CGEventPost(kCGHIDEventTap, click1_up);
                CFRelease(click1_up);
                CFRelease(click1_down);
            #endif
            
        }
        
        #ifdef QBNex_MACOSX
            //128 indicates SHIFT must be held to achieve the indexed ASCII character
            static uint16 ASCII_TO_MACVK[]={
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0x3300+8,
                0x3000+9,
                0,
                0,
                0,
                0x2400+13,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0x3500+27,
                0,
                0,
                0,
                0,
                0x3100+32,
                0x1200+33+128,
                0x2700+34+128,
                0x1400+35+128,
                0x1500+36+128,
                0x1700+37+128,
                0x1A00+38+128,
                0x2700+39,
                0x1900+40+128,
                0x1D00+41+128,
                0x1C00+42+128,
                0x1800+43+128,
                0x2B00+44,
                0x1B00+45,
                0x2F00+46,
                0x2C00+47,
                0x1D00+48,
                0x1200+49,
                0x1300+50,
                0x1400+51,
                0x1500+52,
                0x1700+53,
                0x1600+54,
                0x1A00+55,
                0x1C00+56,
                0x1900+57,
                0x2900+58+128,
                0x2900+59,
                0x2B00+60+128,
                0x1800+61,
                0x2F00+62+128,
                0x2C00+63+128,
                0x1300+64+128,
                0x0000+65+128,
                0x0B00+66+128,
                0x0800+67+128,
                0x0200+68+128,
                0x0E00+69+128,
                0x0300+70+128,
                0x0500+71+128,
                0x0400+72+128,
                0x2200+73+128,
                0x2600+74+128,
                0x2800+75+128,
                0x2500+76+128,
                0x2E00+77+128,
                0x2D00+78+128,
                0x1F00+79+128,
                0x2300+80+128,
                0x0C00+81+128,
                0x0F00+82+128,
                0x0100+83+128,
                0x1100+84+128,
                0x2000+85+128,
                0x0900+86+128,
                0x0D00+87+128,
                0x0700+88+128,
                0x1000+89+128,
                0x0600+90+128,
                0x2100+91,
                0x2A00+92,
                0x1E00+93,
                0x1600+94+128,
                0x1B00+95+128,
                0x3200+96,
                0x0000+65+32,
                0x0B00+66+32,
                0x0800+67+32,
                0x0200+68+32,
                0x0E00+69+32,
                0x0300+70+32,
                0x0500+71+32,
                0x0400+72+32,
                0x2200+73+32,
                0x2600+74+32,
                0x2800+75+32,
                0x2500+76+32,
                0x2E00+77+32,
                0x2D00+78+32,
                0x1F00+79+32,
                0x2300+80+32,
                0x0C00+81+32,
                0x0F00+82+32,
                0x0100+83+32,
                0x1100+84+32,
                0x2000+85+32,
                0x0900+86+32,
                0x0D00+87+32,
                0x0700+88+32,
                0x1000+89+32,
                0x0600+90+32,
                0x2100+123+128,
                0x2A00+124+128,
                0x1E00+125+128,
                0x3200+126+128,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0
            };
        #endif
        
        int32 MACVK_ANSI_A                    = 0x00;
        int32 MACVK_ANSI_S                    = 0x01;
        int32 MACVK_ANSI_D                    = 0x02;
        int32 MACVK_ANSI_F                    = 0x03;
        int32 MACVK_ANSI_H                    = 0x04;
        int32 MACVK_ANSI_G                    = 0x05;
        int32 MACVK_ANSI_Z                    = 0x06;
        int32 MACVK_ANSI_X                    = 0x07;
        int32 MACVK_ANSI_C                    = 0x08;
        int32 MACVK_ANSI_V                    = 0x09;
        int32 MACVK_ANSI_B                    = 0x0B;
        int32 MACVK_ANSI_Q                    = 0x0C;
        int32 MACVK_ANSI_W                    = 0x0D;
        int32 MACVK_ANSI_E                    = 0x0E;
        int32 MACVK_ANSI_R                    = 0x0F;
        int32 MACVK_ANSI_Y                    = 0x10;
        int32 MACVK_ANSI_T                    = 0x11;
        int32 MACVK_ANSI_1                    = 0x12;
        int32 MACVK_ANSI_2                    = 0x13;
        int32 MACVK_ANSI_3                    = 0x14;
        int32 MACVK_ANSI_4                    = 0x15;
        int32 MACVK_ANSI_6                    = 0x16;
        int32 MACVK_ANSI_5                    = 0x17;
        int32 MACVK_ANSI_Equal                = 0x18;
        int32 MACVK_ANSI_9                    = 0x19;
        int32 MACVK_ANSI_7                    = 0x1A;
        int32 MACVK_ANSI_Minus                = 0x1B;
        int32 MACVK_ANSI_8                    = 0x1C;
        int32 MACVK_ANSI_0                    = 0x1D;
        int32 MACVK_ANSI_RightBracket         = 0x1E;
        int32 MACVK_ANSI_O                    = 0x1F;
        int32 MACVK_ANSI_U                    = 0x20;
        int32 MACVK_ANSI_LeftBracket          = 0x21;
        int32 MACVK_ANSI_I                    = 0x22;
        int32 MACVK_ANSI_P                    = 0x23;
        int32 MACVK_ANSI_L                    = 0x25;
        int32 MACVK_ANSI_J                    = 0x26;
        int32 MACVK_ANSI_Quote                = 0x27;
        int32 MACVK_ANSI_K                    = 0x28;
        int32 MACVK_ANSI_Semicolon            = 0x29;
        int32 MACVK_ANSI_Backslash            = 0x2A;
        int32 MACVK_ANSI_Comma                = 0x2B;
        int32 MACVK_ANSI_Slash                = 0x2C;
        int32 MACVK_ANSI_N                    = 0x2D;
        int32 MACVK_ANSI_M                    = 0x2E;
        int32 MACVK_ANSI_Period               = 0x2F;
        int32 MACVK_ANSI_Grave                = 0x32;
        int32 MACVK_ANSI_KeypadDecimal        = 0x41;
        int32 MACVK_ANSI_KeypadMultiply       = 0x43;
        int32 MACVK_ANSI_KeypadPlus           = 0x45;
        int32 MACVK_ANSI_KeypadClear          = 0x47;
        int32 MACVK_ANSI_KeypadDivide         = 0x4B;
        int32 MACVK_ANSI_KeypadEnter          = 0x4C;
        int32 MACVK_ANSI_KeypadMinus          = 0x4E;
        int32 MACVK_ANSI_KeypadEquals         = 0x51;
        int32 MACVK_ANSI_Keypad0              = 0x52;
        int32 MACVK_ANSI_Keypad1              = 0x53;
        int32 MACVK_ANSI_Keypad2              = 0x54;
        int32 MACVK_ANSI_Keypad3              = 0x55;
        int32 MACVK_ANSI_Keypad4              = 0x56;
        int32 MACVK_ANSI_Keypad5              = 0x57;
        int32 MACVK_ANSI_Keypad6              = 0x58;
        int32 MACVK_ANSI_Keypad7              = 0x59;
        int32 MACVK_ANSI_Keypad8              = 0x5B;
        int32 MACVK_ANSI_Keypad9              = 0x5C;
        int32 MACVK_Return                    = 0x24;
        int32 MACVK_Tab                       = 0x30;
        int32 MACVK_Space                     = 0x31;
        int32 MACVK_Delete                    = 0x33;
        int32 MACVK_Escape                    = 0x35;
        int32 MACVK_Command                   = 0x37;
        int32 MACVK_Shift                     = 0x38;
        int32 MACVK_CapsLock                  = 0x39;
        int32 MACVK_Option                    = 0x3A;
        int32 MACVK_Control                   = 0x3B;
        int32 MACVK_RightShift                = 0x3C;
        int32 MACVK_RightOption               = 0x3D;
        int32 MACVK_RightControl              = 0x3E;
        int32 MACVK_Function                  = 0x3F;
        int32 MACVK_F17                       = 0x40;
        int32 MACVK_VolumeUp                  = 0x48;
        int32 MACVK_VolumeDown                = 0x49;
        int32 MACVK_Mute                      = 0x4A;
        int32 MACVK_F18                       = 0x4F;
        int32 MACVK_F19                       = 0x50;
        int32 MACVK_F20                       = 0x5A;
        int32 MACVK_F5                        = 0x60;
        int32 MACVK_F6                        = 0x61;
        int32 MACVK_F7                        = 0x62;
        int32 MACVK_F3                        = 0x63;
        int32 MACVK_F8                        = 0x64;
        int32 MACVK_F9                        = 0x65;
        int32 MACVK_F11                       = 0x67;
        int32 MACVK_F13                       = 0x69;
        int32 MACVK_F16                       = 0x6A;
        int32 MACVK_F14                       = 0x6B;
        int32 MACVK_F10                       = 0x6D;
        int32 MACVK_F12                       = 0x6F;
        int32 MACVK_F15                       = 0x71;
        int32 MACVK_Help                      = 0x72;
        int32 MACVK_Home                      = 0x73;
        int32 MACVK_PageUp                    = 0x74;
        int32 MACVK_ForwardDelete             = 0x75;
        int32 MACVK_F4                        = 0x76;
        int32 MACVK_End                       = 0x77;
        int32 MACVK_F2                        = 0x78;
        int32 MACVK_PageDown                  = 0x79;
        int32 MACVK_F1                        = 0x7A;
        int32 MACVK_LeftArrow                 = 0x7B;
        int32 MACVK_RightArrow                = 0x7C;
        int32 MACVK_DownArrow                 = 0x7D;
        int32 MACVK_UpArrow                   = 0x7E;
        int32 MACVK_ISO_Section               = 0x0A;
        int32 MACVK_JIS_Yen                   = 0x5D;
        int32 MACVK_JIS_Underscore            = 0x5E;
        int32 MACVK_JIS_KeypadComma           = 0x5F;
        int32 MACVK_JIS_Eisu                  = 0x66;
        int32 MACVK_JIS_Kana                  = 0x68;
        
        
        
        void sub__screenprint(qbs *txt){
            
            static int32 i,s,x,vk,c;
            
            #ifdef QBNex_MACOSX
                /* MACOSX virtual key code reference (with ASCII value & shift state):
                    static int32 MACVK_ANSI_A                    = 0x0000+65+32;
                    static int32 MACVK_ANSI_S                    = 0x0100+83+32;
                    static int32 MACVK_ANSI_D                    = 0x0200+68+32;
                    static int32 MACVK_ANSI_F                    = 0x0300+70+32;
                    static int32 MACVK_ANSI_H                    = 0x0400+72+32;
                    static int32 MACVK_ANSI_G                    = 0x0500+71+32;
                    static int32 MACVK_ANSI_Z                    = 0x0600+90+32;
                    static int32 MACVK_ANSI_X                    = 0x0700+88+32;
                    static int32 MACVK_ANSI_C                    = 0x0800+67+32;
                    static int32 MACVK_ANSI_V                    = 0x0900+86+32;
                    static int32 MACVK_ANSI_B                    = 0x0B00+66+32;
                    static int32 MACVK_ANSI_Q                    = 0x0C00+81+32;
                    static int32 MACVK_ANSI_W                    = 0x0D00+87+32;
                    static int32 MACVK_ANSI_E                    = 0x0E00+69+32;
                    static int32 MACVK_ANSI_R                    = 0x0F00+82+32;
                    static int32 MACVK_ANSI_Y                    = 0x1000+89+32;
                    static int32 MACVK_ANSI_T                    = 0x1100+84+32;
                    static int32 MACVK_ANSI_1                    = 0x1200+49;
                    static int32 MACVK_ANSI_2                    = 0x1300+50;
                    static int32 MACVK_ANSI_3                    = 0x1400+51;
                    static int32 MACVK_ANSI_4                    = 0x1500+52;
                    static int32 MACVK_ANSI_6                    = 0x1600+54;
                    static int32 MACVK_ANSI_5                    = 0x1700+53;
                    static int32 MACVK_ANSI_Equal                = 0x1800+61;
                    static int32 MACVK_ANSI_9                    = 0x1900+57;
                    static int32 MACVK_ANSI_7                    = 0x1A00+55;
                    static int32 MACVK_ANSI_Minus                = 0x1B00+45;
                    static int32 MACVK_ANSI_8                    = 0x1C00+56;
                    static int32 MACVK_ANSI_0                    = 0x1D00+48;
                    static int32 MACVK_ANSI_RightBracket         = 0x1E00+93;
                    static int32 MACVK_ANSI_O                    = 0x1F00+79+32;
                    static int32 MACVK_ANSI_U                    = 0x2000+85+32;
                    static int32 MACVK_ANSI_LeftBracket          = 0x2100+91;
                    static int32 MACVK_ANSI_I                    = 0x2200+73+32;
                    static int32 MACVK_ANSI_P                    = 0x2300+80+32;
                    static int32 MACVK_ANSI_L                    = 0x2500+76+32;
                    static int32 MACVK_ANSI_J                    = 0x2600+74+32;
                    static int32 MACVK_ANSI_Quote                = 0x2700+39;
                    static int32 MACVK_ANSI_K                    = 0x2800+75+32;
                    static int32 MACVK_ANSI_Semicolon            = 0x2900+59;
                    static int32 MACVK_ANSI_Backslash            = 0x2A00+92;
                    static int32 MACVK_ANSI_Comma                = 0x2B00+44;
                    static int32 MACVK_ANSI_Slash                = 0x2C00+47;
                    static int32 MACVK_ANSI_N                    = 0x2D00+78+32;
                    static int32 MACVK_ANSI_M                    = 0x2E00+77+32;
                    static int32 MACVK_ANSI_Period               = 0x2F00+46;
                    static int32 MACVK_ANSI_Grave                = 0x3200+96;
                    static int32 MACVK_ANSI_KeypadDecimal        = 0x4100;
                    static int32 MACVK_ANSI_KeypadMultiply       = 0x4300;
                    static int32 MACVK_ANSI_KeypadPlus           = 0x4500;
                    static int32 MACVK_ANSI_KeypadClear          = 0x4700;
                    static int32 MACVK_ANSI_KeypadDivide         = 0x4B00;
                    static int32 MACVK_ANSI_KeypadEnter          = 0x4C00;
                    static int32 MACVK_ANSI_KeypadMinus          = 0x4E00;
                    static int32 MACVK_ANSI_KeypadEquals         = 0x5100;
                    static int32 MACVK_ANSI_Keypad0              = 0x5200;
                    static int32 MACVK_ANSI_Keypad1              = 0x5300;
                    static int32 MACVK_ANSI_Keypad2              = 0x5400;
                    static int32 MACVK_ANSI_Keypad3              = 0x5500;
                    static int32 MACVK_ANSI_Keypad4              = 0x5600;
                    static int32 MACVK_ANSI_Keypad5              = 0x5700;
                    static int32 MACVK_ANSI_Keypad6              = 0x5800;
                    static int32 MACVK_ANSI_Keypad7              = 0x5900;
                    static int32 MACVK_ANSI_Keypad8              = 0x5B00;
                    static int32 MACVK_ANSI_Keypad9              = 0x5C00;
                    static int32 MACVK_Return                    = 0x2400+13;
                    static int32 MACVK_Tab                       = 0x3000+9;
                    static int32 MACVK_Space                     = 0x3100+32;
                    static int32 MACVK_Delete                    = 0x3300+8;
                    static int32 MACVK_Escape                    = 0x3500+27;
                    static int32 MACVK_Command                   = 0x3700;
                    static int32 MACVK_Shift                     = 0x3800;
                    static int32 MACVK_CapsLock                  = 0x3900;
                    static int32 MACVK_Option                    = 0x3A00;
                    static int32 MACVK_Control                   = 0x3B00;
                    static int32 MACVK_RightShift                = 0x3C00;
                    static int32 MACVK_RightOption               = 0x3D00;
                    static int32 MACVK_RightControl              = 0x3E00;
                    static int32 MACVK_Function                  = 0x3F00;
                    static int32 MACVK_F17                       = 0x4000;
                    static int32 MACVK_VolumeUp                  = 0x4800;
                    static int32 MACVK_VolumeDown                = 0x4900;
                    static int32 MACVK_Mute                      = 0x4A00;
                    static int32 MACVK_F18                       = 0x4F00;
                    static int32 MACVK_F19                       = 0x5000;
                    static int32 MACVK_F20                       = 0x5A00;
                    static int32 MACVK_F5                        = 0x6000;
                    static int32 MACVK_F6                        = 0x6100;
                    static int32 MACVK_F7                        = 0x6200;
                    static int32 MACVK_F3                        = 0x6300;
                    static int32 MACVK_F8                        = 0x6400;
                    static int32 MACVK_F9                        = 0x6500;
                    static int32 MACVK_F11                       = 0x6700;
                    static int32 MACVK_F13                       = 0x6900;
                    static int32 MACVK_F16                       = 0x6A00;
                    static int32 MACVK_F14                       = 0x6B00;
                    static int32 MACVK_F10                       = 0x6D00;
                    static int32 MACVK_F12                       = 0x6F00;
                    static int32 MACVK_F15                       = 0x7100;
                    static int32 MACVK_Help                      = 0x7200;
                    static int32 MACVK_Home                      = 0x7300;
                    static int32 MACVK_PageUp                    = 0x7400;
                    static int32 MACVK_ForwardDelete             = 0x7500;
                    static int32 MACVK_F4                        = 0x7600;
                    static int32 MACVK_End                       = 0x7700;
                    static int32 MACVK_F2                        = 0x7800;
                    static int32 MACVK_PageDown                  = 0x7900;
                    static int32 MACVK_F1                        = 0x7A00;
                    static int32 MACVK_LeftArrow                 = 0x7B00;
                    static int32 MACVK_RightArrow                = 0x7C00;
                    static int32 MACVK_DownArrow                 = 0x7D00;
                    static int32 MACVK_UpArrow                   = 0x7E00;
                    static int32 MACVK_ISO_Section               = 0x0A00;
                    static int32 MACVK_JIS_Yen                   = 0x5D00;
                    static int32 MACVK_JIS_Underscore            = 0x5E00;
                    static int32 MACVK_JIS_KeypadComma           = 0x5F00;
                    static int32 MACVK_JIS_Eisu                  = 0x6600;
                    static int32 MACVK_JIS_Kana                  = 0x6800;
                    static int32 MACVKS_ANSI_A                    = 0x0000+65+128;
                    static int32 MACVKS_ANSI_S                    = 0x0100+83+128;
                    static int32 MACVKS_ANSI_D                    = 0x0200+68+128;
                    static int32 MACVKS_ANSI_F                    = 0x0300+70+128;
                    static int32 MACVKS_ANSI_H                    = 0x0400+72+128;
                    static int32 MACVKS_ANSI_G                    = 0x0500+71+128;
                    static int32 MACVKS_ANSI_Z                    = 0x0600+90+128;
                    static int32 MACVKS_ANSI_X                    = 0x0700+88+128;
                    static int32 MACVKS_ANSI_C                    = 0x0800+67+128;
                    static int32 MACVKS_ANSI_V                    = 0x0900+86+128;
                    static int32 MACVKS_ANSI_B                    = 0x0B00+66+128;
                    static int32 MACVKS_ANSI_Q                    = 0x0C00+81+128;
                    
                    static int32 MACVKS_ANSI_W                    = 0x0D00+87+128;
                    static int32 MACVKS_ANSI_E                    = 0x0E00+69+128;
                    static int32 MACVKS_ANSI_R                    = 0x0F00+82+128;
                    static int32 MACVKS_ANSI_Y                    = 0x1000+89+128;
                    static int32 MACVKS_ANSI_T                    = 0x1100+84+128;
                    static int32 MACVKS_ANSI_1                    = 0x1200+33+128;
                    static int32 MACVKS_ANSI_2                    = 0x1300+64+128;
                    static int32 MACVKS_ANSI_3                    = 0x1400+35+128;
                    static int32 MACVKS_ANSI_4                    = 0x1500+36+128;
                    static int32 MACVKS_ANSI_6                    = 0x1600+94+128;
                    static int32 MACVKS_ANSI_5                    = 0x1700+37+128;
                    static int32 MACVKS_ANSI_Equal                = 0x1800+43+128;
                    static int32 MACVKS_ANSI_9                    = 0x1900+40+128;
                    static int32 MACVKS_ANSI_7                    = 0x1A00+38+128;
                    static int32 MACVKS_ANSI_Minus                = 0x1B00+95+128;
                    static int32 MACVKS_ANSI_8                    = 0x1C00+42+128;
                    static int32 MACVKS_ANSI_0                    = 0x1D00+41+128;
                    static int32 MACVKS_ANSI_RightBracket         = 0x1E00+125+128;
                    static int32 MACVKS_ANSI_O                    = 0x1F00+79+128;
                    static int32 MACVKS_ANSI_U                    = 0x2000+85+128;
                    static int32 MACVKS_ANSI_LeftBracket          = 0x2100+123+128;
                    static int32 MACVKS_ANSI_I                    = 0x2200+73+128;
                    static int32 MACVKS_ANSI_P                    = 0x2300+80+128;
                    static int32 MACVKS_ANSI_L                    = 0x2500+76+128;
                    static int32 MACVKS_ANSI_J                    = 0x2600+74+128;
                    static int32 MACVKS_ANSI_Quote                = 0x2700+34+128;
                    static int32 MACVKS_ANSI_K                    = 0x2800+75+128;
                    static int32 MACVKS_ANSI_Semicolon            = 0x2900+58+128;
                    static int32 MACVKS_ANSI_Backslash            = 0x2A00+124+128;
                    static int32 MACVKS_ANSI_Comma                = 0x2B00+60+128;
                    static int32 MACVKS_ANSI_Slash                = 0x2C00+63+128;
                    static int32 MACVKS_ANSI_N                    = 0x2D00+78+128;
                    static int32 MACVKS_ANSI_M                    = 0x2E00+77+128;
                    static int32 MACVKS_ANSI_Period               = 0x2F00+62+128;
                    static int32 MACVKS_ANSI_Grave                = 0x3200+126+128;
                */
                
                
                static CGEventSourceRef es;
                static CGEventRef e;
                
                for (i=0;i<txt->len;i++){
                    c=txt->chr[i];
                    
                    //static int32 i,s,x,vk,c;
                    
                    /*
                        CONTROL+{A-Z} 
                        The following 'x' letters cannot be simulated this way because they map to implemented control code (8,9,13) functionality:
                        ABCDEFGHIJKLMNOPQRSTUVWXYZ
                        .......xx...x.............
                        Common/standard CTRL+? combinations for copying, pasting, undoing, cutting, etc. are available
                    */
                    
                    if ((c>=1)&&(c<=26)){ if ((c!=8)&&(c!=9)&&(c!=13)){
                        //Note: Under MacOSX, COMMAND is used instead of control for general tasks
                        vk=ASCII_TO_MACVK[c+96]>>8;
                        es=CGEventSourceCreate(kCGEventSourceStateCombinedSessionState);
                        e=CGEventCreateKeyboardEvent(es,(CGKeyCode)MACVK_Command,1); CGEventPost(kCGAnnotatedSessionEventTap,e); CFRelease(e);
                        e=CGEventCreateKeyboardEvent(es,(CGKeyCode)vk,1); CGEventSetFlags(e, kCGEventFlagMaskCommand);
                        CGEventPost(kCGAnnotatedSessionEventTap,e); CFRelease(e);
                        e=CGEventCreateKeyboardEvent(es,(CGKeyCode)vk,0); CGEventPost(kCGAnnotatedSessionEventTap,e); CFRelease(e);
                        e=CGEventCreateKeyboardEvent(es,(CGKeyCode)MACVK_Command,0); CGEventPost(kCGAnnotatedSessionEventTap,e); CFRelease(e);
                        goto special_character;
                    }}
                    
                    //custom extended characters
                    if (c==0){
                        if (i==(txt->len-1)) goto special_character;
                        i++;
                        c=txt->chr[i];
                        if (c==15){//SHIFT+TAB
                            vk=MACVK_Tab;
                            es=CGEventSourceCreate(kCGEventSourceStateCombinedSessionState);
                            e=CGEventCreateKeyboardEvent(es,(CGKeyCode)MACVK_Shift,1); CGEventPost(kCGAnnotatedSessionEventTap,e); CFRelease(e);
                            e=CGEventCreateKeyboardEvent(es,(CGKeyCode)vk,1); CGEventSetFlags(e, kCGEventFlagMaskShift);
                            CGEventPost(kCGAnnotatedSessionEventTap,e); CFRelease(e);
                            e=CGEventCreateKeyboardEvent(es,(CGKeyCode)vk,0); CGEventPost(kCGAnnotatedSessionEventTap,e); CFRelease(e);
                            e=CGEventCreateKeyboardEvent(es,(CGKeyCode)MACVK_Shift,0); CGEventPost(kCGAnnotatedSessionEventTap,e); CFRelease(e);
                            CFRelease(es);
                        }
                        //4 arrows
                        if (c==75){
                            vk=MACVK_LeftArrow;
                            es=CGEventSourceCreate(kCGEventSourceStateCombinedSessionState);
                            e=CGEventCreateKeyboardEvent(es,(CGKeyCode)vk,1); CGEventPost(kCGAnnotatedSessionEventTap,e); CFRelease(e);
                            e=CGEventCreateKeyboardEvent(es,(CGKeyCode)vk,0); CGEventPost(kCGAnnotatedSessionEventTap,e); CFRelease(e);
                            CFRelease(es);
                        }
                        if (c==77){
                            vk=MACVK_RightArrow;
                            es=CGEventSourceCreate(kCGEventSourceStateCombinedSessionState);
                            e=CGEventCreateKeyboardEvent(es,(CGKeyCode)vk,1); CGEventPost(kCGAnnotatedSessionEventTap,e); CFRelease(e);
                            e=CGEventCreateKeyboardEvent(es,(CGKeyCode)vk,0); CGEventPost(kCGAnnotatedSessionEventTap,e); CFRelease(e);
                            CFRelease(es);
                        }
                        if (c==72){
                            vk=MACVK_UpArrow;
                            es=CGEventSourceCreate(kCGEventSourceStateCombinedSessionState);
                            e=CGEventCreateKeyboardEvent(es,(CGKeyCode)vk,1); CGEventPost(kCGAnnotatedSessionEventTap,e); CFRelease(e);
                            e=CGEventCreateKeyboardEvent(es,(CGKeyCode)vk,0); CGEventPost(kCGAnnotatedSessionEventTap,e); CFRelease(e);
                            CFRelease(es);
                        }
                        if (c==80){
                            vk=MACVK_DownArrow;
                            es=CGEventSourceCreate(kCGEventSourceStateCombinedSessionState);
                            e=CGEventCreateKeyboardEvent(es,(CGKeyCode)vk,1); CGEventPost(kCGAnnotatedSessionEventTap,e); CFRelease(e);
                            e=CGEventCreateKeyboardEvent(es,(CGKeyCode)vk,0); CGEventPost(kCGAnnotatedSessionEventTap,e); CFRelease(e);
                            CFRelease(es);
                        }
                        //6 control keys
                        if (c==82){
                            vk=MACVK_Help;
                            es=CGEventSourceCreate(kCGEventSourceStateCombinedSessionState);
                            e=CGEventCreateKeyboardEvent(es,(CGKeyCode)vk,1); CGEventPost(kCGAnnotatedSessionEventTap,e); CFRelease(e);
                            e=CGEventCreateKeyboardEvent(es,(CGKeyCode)vk,0); CGEventPost(kCGAnnotatedSessionEventTap,e); CFRelease(e);
                            CFRelease(es);
                        }
                        if (c==71){
                            vk=MACVK_Home;
                            es=CGEventSourceCreate(kCGEventSourceStateCombinedSessionState);
                            e=CGEventCreateKeyboardEvent(es,(CGKeyCode)vk,1); CGEventPost(kCGAnnotatedSessionEventTap,e); CFRelease(e);
                            e=CGEventCreateKeyboardEvent(es,(CGKeyCode)vk,0); CGEventPost(kCGAnnotatedSessionEventTap,e); CFRelease(e);
                            CFRelease(es);
                        }
                        if (c==83){
                            vk=MACVK_ForwardDelete;
                            es=CGEventSourceCreate(kCGEventSourceStateCombinedSessionState);
                            e=CGEventCreateKeyboardEvent(es,(CGKeyCode)vk,1); CGEventPost(kCGAnnotatedSessionEventTap,e); CFRelease(e);
                            e=CGEventCreateKeyboardEvent(es,(CGKeyCode)vk,0); CGEventPost(kCGAnnotatedSessionEventTap,e); CFRelease(e);
                            CFRelease(es);
                        }
                        if (c==79){
                            vk=MACVK_End;
                            es=CGEventSourceCreate(kCGEventSourceStateCombinedSessionState);
                            e=CGEventCreateKeyboardEvent(es,(CGKeyCode)vk,1); CGEventPost(kCGAnnotatedSessionEventTap,e); CFRelease(e);
                            e=CGEventCreateKeyboardEvent(es,(CGKeyCode)vk,0); CGEventPost(kCGAnnotatedSessionEventTap,e); CFRelease(e);
                            CFRelease(es);
                        }
                        if (c==81){
                            vk=MACVK_PageDown;
                            es=CGEventSourceCreate(kCGEventSourceStateCombinedSessionState);
                            e=CGEventCreateKeyboardEvent(es,(CGKeyCode)vk,1); CGEventPost(kCGAnnotatedSessionEventTap,e); CFRelease(e);
                            e=CGEventCreateKeyboardEvent(es,(CGKeyCode)vk,0); CGEventPost(kCGAnnotatedSessionEventTap,e); CFRelease(e);
                            CFRelease(es);
                        }
                        if (c==73){
                            vk=MACVK_PageUp;
                            es=CGEventSourceCreate(kCGEventSourceStateCombinedSessionState);
                            e=CGEventCreateKeyboardEvent(es,(CGKeyCode)vk,1); CGEventPost(kCGAnnotatedSessionEventTap,e); CFRelease(e);
                            e=CGEventCreateKeyboardEvent(es,(CGKeyCode)vk,0); CGEventPost(kCGAnnotatedSessionEventTap,e); CFRelease(e);
                            CFRelease(es);
                        }
                        //...
                        //todo: F1-F12, shift/control/alt+above
                        goto special_character;
                    }
                    
                    //standard ASCII character output
                    x=ASCII_TO_MACVK[c];
                    if (x&127){//available character
                        vk=x>>8;
                        if (x&128){
                            es=CGEventSourceCreate(kCGEventSourceStateCombinedSessionState);
                            e=CGEventCreateKeyboardEvent(es,(CGKeyCode)MACVK_Shift,1); CGEventPost(kCGAnnotatedSessionEventTap,e); CFRelease(e);
                            e=CGEventCreateKeyboardEvent(es,(CGKeyCode)vk,1); CGEventSetFlags(e, kCGEventFlagMaskShift);
                            CGEventPost(kCGAnnotatedSessionEventTap,e); CFRelease(e);
                            e=CGEventCreateKeyboardEvent(es,(CGKeyCode)vk,0); CGEventPost(kCGAnnotatedSessionEventTap,e); CFRelease(e);
                            e=CGEventCreateKeyboardEvent(es,(CGKeyCode)MACVK_Shift,0); CGEventPost(kCGAnnotatedSessionEventTap,e); CFRelease(e);
                            CFRelease(es);
                            }else{
                            es=CGEventSourceCreate(kCGEventSourceStateCombinedSessionState);
                            e=CGEventCreateKeyboardEvent(es,(CGKeyCode)vk,1); CGEventPost(kCGAnnotatedSessionEventTap,e); CFRelease(e);
                            e=CGEventCreateKeyboardEvent(es,(CGKeyCode)vk,0); CGEventPost(kCGAnnotatedSessionEventTap,e); CFRelease(e);
                            CFRelease(es);
                        }
                    }//available character
                    
                    special_character:;
                    
                }//i
                
            #endif //QBNex_MACOSX
            
            
            
            #ifdef QBNex_WINDOWS
                
                static INPUT input;
                
                /*VK reference:
                    http://msdn.microsoft.com/en-us/library/ms927178.aspx
                */
                
                for (i=0;i<txt->len;i++){
                    c=txt->chr[i];
                    //custom characters
                    if (c==9){ZeroMemory(&input,sizeof(INPUT)); input.ki.wVk=VK_TAB; input.type=INPUT_KEYBOARD; SendInput(1,&input,sizeof(INPUT)); input.ki.dwFlags = KEYEVENTF_KEYUP; SendInput(1,&input,sizeof(INPUT)); goto special_character;}
                    if (c==8){ZeroMemory(&input,sizeof(INPUT)); input.ki.wVk=VK_BACK; input.type=INPUT_KEYBOARD; SendInput(1,&input,sizeof(INPUT)); input.ki.dwFlags = KEYEVENTF_KEYUP; SendInput(1,&input,sizeof(INPUT)); goto special_character;}
                    if (c==13){ZeroMemory(&input,sizeof(INPUT)); input.ki.wVk=VK_RETURN; input.type=INPUT_KEYBOARD; SendInput(1,&input,sizeof(INPUT)); input.ki.dwFlags = KEYEVENTF_KEYUP; SendInput(1,&input,sizeof(INPUT)); goto special_character;}
                    //...
                    
                    /*
                        CONTROL+{A-Z} 
                        The following 'x' letters cannot be simulated this way because they map to above functionality:
                        ABCDEFGHIJKLMNOPQRSTUVWXYZ
                        .......xx...x.............
                        Common/standard CTRL+? combinations for copying, pasting, undoing, cutting, etc. are available
                    */
                    if ((c>=1)&&(c<=26)){
                        ZeroMemory(&input,sizeof(INPUT)); input.type=INPUT_KEYBOARD; input.ki.wVk=VK_CONTROL; SendInput(1,&input,sizeof(INPUT));
                        ZeroMemory(&input,sizeof(INPUT)); input.ki.wVk=VkKeyScan(64+c)&255; input.type=INPUT_KEYBOARD; SendInput(1,&input,sizeof(INPUT)); input.ki.dwFlags = KEYEVENTF_KEYUP; SendInput(1,&input,sizeof(INPUT));
                        ZeroMemory(&input,sizeof(INPUT)); input.type=INPUT_KEYBOARD; input.ki.wVk=VK_CONTROL; input.ki.dwFlags = KEYEVENTF_KEYUP; SendInput(1,&input,sizeof(INPUT));
                        goto special_character;
                    }
                    
                    //custom extended characters
                    if (c==0){
                        if (i==(txt->len-1)) goto special_character;
                        i++;
                        c=txt->chr[i];
                        if (c==15){//SHIFT+TAB
                            ZeroMemory(&input,sizeof(INPUT)); input.type=INPUT_KEYBOARD; input.ki.wVk=VK_SHIFT; SendInput(1,&input,sizeof(INPUT));
                            ZeroMemory(&input,sizeof(INPUT)); input.ki.wVk=VK_TAB; input.type=INPUT_KEYBOARD; SendInput(1,&input,sizeof(INPUT)); input.ki.dwFlags = KEYEVENTF_KEYUP; SendInput(1,&input,sizeof(INPUT));
                            ZeroMemory(&input,sizeof(INPUT)); input.type=INPUT_KEYBOARD; input.ki.wVk=VK_SHIFT; input.ki.dwFlags = KEYEVENTF_KEYUP; SendInput(1,&input,sizeof(INPUT));
                        }
                        if (c==75){ZeroMemory(&input,sizeof(INPUT)); input.ki.wVk=VK_LEFT; input.type=INPUT_KEYBOARD; SendInput(1,&input,sizeof(INPUT)); input.ki.dwFlags = KEYEVENTF_KEYUP; SendInput(1,&input,sizeof(INPUT));}
                        if (c==77){ZeroMemory(&input,sizeof(INPUT)); input.ki.wVk=VK_RIGHT; input.type=INPUT_KEYBOARD; SendInput(1,&input,sizeof(INPUT)); input.ki.dwFlags = KEYEVENTF_KEYUP; SendInput(1,&input,sizeof(INPUT));}
                        if (c==72){ZeroMemory(&input,sizeof(INPUT)); input.ki.wVk=VK_UP; input.type=INPUT_KEYBOARD; SendInput(1,&input,sizeof(INPUT)); input.ki.dwFlags = KEYEVENTF_KEYUP; SendInput(1,&input,sizeof(INPUT));}
                        if (c==80){ZeroMemory(&input,sizeof(INPUT)); input.ki.wVk=VK_DOWN; input.type=INPUT_KEYBOARD; SendInput(1,&input,sizeof(INPUT)); input.ki.dwFlags = KEYEVENTF_KEYUP; SendInput(1,&input,sizeof(INPUT));}
                        if (c==82){ZeroMemory(&input,sizeof(INPUT)); input.ki.wVk=VK_INSERT; input.type=INPUT_KEYBOARD; SendInput(1,&input,sizeof(INPUT)); input.ki.dwFlags = KEYEVENTF_KEYUP; SendInput(1,&input,sizeof(INPUT));}
                        if (c==71){ZeroMemory(&input,sizeof(INPUT)); input.ki.wVk=VK_HOME; input.type=INPUT_KEYBOARD; SendInput(1,&input,sizeof(INPUT)); input.ki.dwFlags = KEYEVENTF_KEYUP; SendInput(1,&input,sizeof(INPUT));}
                        if (c==83){ZeroMemory(&input,sizeof(INPUT)); input.ki.wVk=VK_DELETE; input.type=INPUT_KEYBOARD; SendInput(1,&input,sizeof(INPUT)); input.ki.dwFlags = KEYEVENTF_KEYUP; SendInput(1,&input,sizeof(INPUT));}
                        if (c==79){ZeroMemory(&input,sizeof(INPUT)); input.ki.wVk=VK_END; input.type=INPUT_KEYBOARD; SendInput(1,&input,sizeof(INPUT)); input.ki.dwFlags = KEYEVENTF_KEYUP; SendInput(1,&input,sizeof(INPUT));}
                        if (c==81){ZeroMemory(&input,sizeof(INPUT)); input.ki.wVk=VK_NEXT; input.type=INPUT_KEYBOARD; SendInput(1,&input,sizeof(INPUT)); input.ki.dwFlags = KEYEVENTF_KEYUP; SendInput(1,&input,sizeof(INPUT));}
                        if (c==73){ZeroMemory(&input,sizeof(INPUT)); input.ki.wVk=VK_PRIOR; input.type=INPUT_KEYBOARD; SendInput(1,&input,sizeof(INPUT)); input.ki.dwFlags = KEYEVENTF_KEYUP; SendInput(1,&input,sizeof(INPUT));}
                        //...
                        //todo: F1-F12, shift/control/alt+above
                        
                        goto special_character;
                    }
                    
                    if ((c>126)||(c<32)) goto special_character;
                    
                    x=VkKeyScan(txt->chr[i]);
                    vk=x&255;
                    
                    s=(x>>8)&255;
                    //1 Either shift key is pressed. 
                    //2 Either CTRL key is pressed. 
                    //4 Either ALT key is pressed. 
                    if (s&1){
                        ZeroMemory(&input,sizeof(INPUT));
                        input.type=INPUT_KEYBOARD;
                        input.ki.wVk=VK_SHIFT;
                        SendInput(1,&input,sizeof(INPUT));
                    }
                    
                    ZeroMemory(&input,sizeof(INPUT));
                    input.type=INPUT_KEYBOARD;
                    input.ki.wVk=vk;
                    SendInput(1,&input,sizeof(INPUT));
                    
                    ZeroMemory(&input,sizeof(INPUT));
                    input.type=INPUT_KEYBOARD;
                    input.ki.wVk=vk;
                    input.ki.dwFlags = KEYEVENTF_KEYUP;
                    SendInput(1,&input,sizeof(INPUT));
                    
                    if (s&1){
                        ZeroMemory(&input,sizeof(INPUT));
                        input.type=INPUT_KEYBOARD;
                        input.ki.wVk=VK_SHIFT;
                        input.ki.dwFlags = KEYEVENTF_KEYUP;
                        SendInput(1,&input,sizeof(INPUT));
                    }
                    
                    special_character:;
                    
                }//i
                
            #endif
            
        }
