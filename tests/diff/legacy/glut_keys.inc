// VERBATIM copy of GLUT_key_ascii + GLUT_key_special from the legacy libqb.cpp (oracle 545b859, lines 27791-27950). Test-only.
        void GLUT_key_ascii(int32 key,int32 down){
            #ifdef QBNex_GLUT
                static int32 v;
                
                static int32 mod;
                mod=glutGetModifiers();//shift=1, control=2, alt=4
                
                #ifndef CORE_FREEGLUT
                    /*
                        if (mod&GLUT_ACTIVE_SHIFT){
                        keydown_vk(VK+QBVK_LSHIFT);
                        }else{
                        keyup_vk(VK+QBVK_LSHIFT);
                        }
                        
                        if (mod&GLUT_ACTIVE_CTRL){
                        keydown_vk(VK+QBVK_LCTRL);
                        }else{
                        keyup_vk(VK+QBVK_LCTRL);
                        }
                        
                        if (mod&GLUT_ACTIVE_ALT){
                        keydown_vk(VK+QBVK_LALT);
                        }else{
                        keyup_vk(VK+QBVK_LALT);
                        }
                    */
                #endif 
                
                //Note: The following is required regardless of whether FREEGLUT is/isn't being used          
                //#ifdef CORE_FREEGLUT          
                //Is CTRL key down? If so, unencode character (applying shift as required)
                if (mod&2){
                    //if (key==127){ //Removed: Might clash with CTRL+DELETE
                    // key=8;
                    // goto ctrl_mod;
                //}//127
                //if (key==3){//CTRL+(BREAK|SCROLL-LOCK)
                //if (down) keydown_vk(VK+QBVK_BREAK); else keyup_vk(VK+QBVK_BREAK);
                //return;
            //}
            if (key==10){
                key=13;
                goto ctrl_mod;
            }//10
            if ((key>=1)&&(key<=26)){
                if (mod&1) key=key-1+65; else key=key-1+97;//assume caps lock off
                goto ctrl_mod;
            }//1-26
        }
        ctrl_mod:
        //#endif
        
        #ifdef QBNex_MACOSX
            
            //swap DEL and backspace keys
            
            if (key==8){
                key=127;
                }else{
                if (key==127){
                    key=8;
                }
            }
            
        #endif
        
        if (key==127){//delete
            if (down) keydown_vk(0x5300); else keyup_vk(0x5300);
            return;
        }
        if (down) keydown_ascii(key); else keyup_ascii(key);
    #endif
}

void GLUT_key_special(int32 key,int32 down){
    
    #ifdef QBNex_GLUT
        #ifndef CORE_FREEGLUT
            /*
                static int32 mod;
                mod=glutGetModifiers();//shift=1, control=2, alt=4
                if (mod&GLUT_ACTIVE_SHIFT){
                keydown_vk(VK+QBVK_LSHIFT);
                }else{
                keyup_vk(VK+QBVK_LSHIFT);
                }
                
                if (mod&GLUT_ACTIVE_CTRL){
                keydown_vk(VK+QBVK_LCTRL);
                }else{
                keyup_vk(VK+QBVK_LCTRL);
                }
                
                if (mod&GLUT_ACTIVE_ALT){
                keydown_vk(VK+QBVK_LALT);
                }else{
                keyup_vk(VK+QBVK_LALT);
                }
            */
        #endif
        
        static int32 vk;
        vk=-1;
        if (key==GLUT_KEY_F1){vk=0x3B00;}
        if (key==GLUT_KEY_F2){vk=0x3C00;}
        if (key==GLUT_KEY_F3){vk=0x3D00;}
        if (key==GLUT_KEY_F4){vk=0x3E00;}
        if (key==GLUT_KEY_F5){vk=0x3F00;}
        if (key==GLUT_KEY_F6){vk=0x4000;}
        if (key==GLUT_KEY_F7){vk=0x4100;}
        if (key==GLUT_KEY_F8){vk=0x4200;}
        if (key==GLUT_KEY_F9){vk=0x4300;}
        if (key==GLUT_KEY_F10){vk=0x4400;}
        if (key==GLUT_KEY_F11){vk=0x8500;}
        if (key==GLUT_KEY_F12){vk=0x8600;}
        if (key==GLUT_KEY_LEFT){vk=0x4B00;}
        if (key==GLUT_KEY_UP){vk=0x4800;}
        if (key==GLUT_KEY_RIGHT){vk=0x4D00;}
        if (key==GLUT_KEY_DOWN){vk=0x5000;}
        if (key==GLUT_KEY_PAGE_UP){vk=0x4900;}
        if (key==GLUT_KEY_PAGE_DOWN){vk=0x5100;}
        if (key==GLUT_KEY_HOME){vk=0x4700;}
        if (key==GLUT_KEY_END){vk=0x4F00;}
        if (key==GLUT_KEY_INSERT){vk=0x5200;}
        
        #ifdef CORE_FREEGLUT
            if (key==112){vk=VK+QBVK_LSHIFT;}
            if (key==113){vk=VK+QBVK_RSHIFT;}
            if (key==114){vk=VK+QBVK_LCTRL;}
            if (key==115){vk=VK+QBVK_RCTRL;}
            if (key==116){vk=VK+QBVK_LALT;}
            if (key==117){vk=VK+QBVK_RALT;}
        #endif
        
        if (vk!=-1){
            if (down) keydown_vk(vk); else keyup_vk(vk);
        }
        
    #endif
}

