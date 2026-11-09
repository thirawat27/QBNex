// Verbatim copy of the legacy COMMAND$/SHELL/KILL/NAME/CHDIR/MKDIR/RMDIR/ENVIRON/FILES/_FILEEXISTS/_DIREXISTS/_CWD$
// code from libqb.cpp (commit 545b859), test-only. Line ranges: 17764-19347, 22562-22653, 27300-27345, 28149-28190, 25922-26029.
// ---- lines 17764,19347
    qbs *func_command_str=NULL;
    char **func_command_array = NULL;
    int32 func_command_count = 0;
    
    qbs *func_command(int32 index, int32 passed){
        static qbs *tqbs;
        if (passed) { //Get specific parameter
            //If out of bounds or error getting cmdline args, return empty string.
            if (index >= func_command_count || index < 0 || func_command_array==NULL) {tqbs = qbs_new(0, 1); return tqbs;}
            int len = strlen(func_command_array[index]);
            //Create new temp qbs and copy data into it.
            tqbs=qbs_new(len, 1);
            memcpy(tqbs->chr, func_command_array[index], len);
        }
        else { //Legacy support; return whole commandline
            tqbs=qbs_new(func_command_str->len,1);
            memcpy(tqbs->chr,func_command_str->chr,func_command_str->len);
        }
        return tqbs;
    }
    
    int32 func__commandcount(){
        return func_command_count - 1;
    }
    
    int32 shell_call_in_progress=0;
    
    #ifdef QBNex_WINDOWS
        int32 cmd_available=-1;
        int32 cmd_ok(){
            if (cmd_available==-1){
                static STARTUPINFO si;
                static PROCESS_INFORMATION pi;
                ZeroMemory( &si, sizeof(si) ); si.cb = sizeof(si); ZeroMemory( &pi, sizeof(pi) );
                if(
                CreateProcess(
                NULL,           // No module name (use command line)
                "cmd.exe /c ver",// Command line
                NULL,           // Process handle not inheritable
                NULL,           // Thread handle not inheritable
                FALSE,          // Set handle inheritance to FALSE
                CREATE_NO_WINDOW, // No creation flags
                NULL,           // Use parent's environment block
                NULL,           // Use parent's starting directory 
                &si,            // Pointer to STARTUPINFO structure
                &pi             // Pointer to PROCESS_INFORMATION structure
                )
                ){
                    WaitForSingleObject(pi.hProcess,INFINITE);
                    CloseHandle(pi.hProcess);
                    CloseHandle(pi.hThread);
                    cmd_available=1;
                    }else{
                    cmd_available=0;
                }
            }//-1
            return cmd_available;
        }
    #endif
    
    
    
    
    
    
    int32 cmd_command(qbs *str2){
        static qbs *str=NULL;
        static int32 s;
        if (!str) str=qbs_new(0,0);
        qbs_set(str,qbs_ucase(str2));
        s=0;
        if (qbs_equal(str,qbs_new_txt("ASSOC")))s=1;
        if (qbs_equal(str,qbs_new_txt("BREAK")))s=1;
        if (qbs_equal(str,qbs_new_txt("BCDBOOT")))s=1;
        if (qbs_equal(str,qbs_new_txt("BCDEDIT")))s=1;
        if (qbs_equal(str,qbs_new_txt("CALL")))s=1;
        if (qbs_equal(str,qbs_new_txt("CD")))s=1;
        if (qbs_equal(str,qbs_new_txt("CHDIR")))s=1;
        if (qbs_equal(str,qbs_new_txt("CLS")))s=1;
        if (qbs_equal(str,qbs_new_txt("COLOR")))s=1;
        if (qbs_equal(str,qbs_new_txt("COPY")))s=1;
        if (qbs_equal(str,qbs_new_txt("DATE")))s=1;
        if (qbs_equal(str,qbs_new_txt("DEFRAG")))s=1;
        if (qbs_equal(str,qbs_new_txt("DEL")))s=1;
        if (qbs_equal(str,qbs_new_txt("DIR")))s=1;
        if (qbs_equal(str,qbs_new_txt("ECHO")))s=1;
        if (qbs_equal(str,qbs_new_txt("ENDLOCAL")))s=1;
        if (qbs_equal(str,qbs_new_txt("ERASE")))s=1;
        if (qbs_equal(str,qbs_new_txt("FOR")))s=1;
        if (qbs_equal(str,qbs_new_txt("FTYPE")))s=1;
        if (qbs_equal(str,qbs_new_txt("GOTO")))s=1;
        if (qbs_equal(str,qbs_new_txt("GRAFTABL")))s=1;
        if (qbs_equal(str,qbs_new_txt("IF")))s=1;
        if (qbs_equal(str,qbs_new_txt("MD")))s=1;
        if (qbs_equal(str,qbs_new_txt("MKDIR")))s=1;
        if (qbs_equal(str,qbs_new_txt("MKLINK")))s=1;
        if (qbs_equal(str,qbs_new_txt("MOVE")))s=1;
        if (qbs_equal(str,qbs_new_txt("PATH")))s=1;
        if (qbs_equal(str,qbs_new_txt("PAUSE")))s=1;
        if (qbs_equal(str,qbs_new_txt("POPD")))s=1;
        if (qbs_equal(str,qbs_new_txt("PROMPT")))s=1;
        if (qbs_equal(str,qbs_new_txt("PUSHD")))s=1;
        if (qbs_equal(str,qbs_new_txt("RD")))s=1;
        if (qbs_equal(str,qbs_new_txt("REM")))s=1;
        if (qbs_equal(str,qbs_new_txt("REN")))s=1;
        if (qbs_equal(str,qbs_new_txt("RENAME")))s=1;
        if (qbs_equal(str,qbs_new_txt("RMDIR")))s=1;
        if (qbs_equal(str,qbs_new_txt("SET")))s=1;
        if (qbs_equal(str,qbs_new_txt("SETLOCAL")))s=1;
        if (qbs_equal(str,qbs_new_txt("SHIFT")))s=1;
        if (qbs_equal(str,qbs_new_txt("START")))s=1;
        if (qbs_equal(str,qbs_new_txt("TIME")))s=1;
        if (qbs_equal(str,qbs_new_txt("TITLE")))s=1;
        if (qbs_equal(str,qbs_new_txt("TYPE")))s=1;
        if (qbs_equal(str,qbs_new_txt("VER")))s=1;
        if (qbs_equal(str,qbs_new_txt("VERIFY")))s=1;
        if (qbs_equal(str,qbs_new_txt("VOL")))s=1;
        return s;
    }
    
    int64 func_shell(qbs *str){
        if (new_error) return 1;
        
        int64 return_code;
        
        //exit full screen mode if necessary
        static int32 full_screen_mode;
        full_screen_mode=full_screen;
        if (full_screen_mode){
            full_screen_set=0;
            do{
                Sleep(0);
            }while(full_screen);
        }//full_screen_mode
        static qbs *strz=NULL;
        static qbs *str1=NULL;
        static qbs *str1z=NULL;
        static qbs *str2=NULL;
        static qbs *str2z=NULL;
        static int32 i;
        
        static int32 use_console;
        use_console=0;
        if (console){
            if (console_active){
                use_console=1;
            }
        }
        
        if (!strz) strz=qbs_new(0,0);
        if (!str1) str1=qbs_new(0,0);
        if (!str1z) str1z=qbs_new(0,0);
        if (!str2) str2=qbs_new(0,0);
        if (!str2z) str2z=qbs_new(0,0);
        
        if (str->len){
            
            #ifdef QBNex_WINDOWS
                
                
                if (use_console){
                    qbs_set(strz,qbs_add(str,qbs_new_txt_len("\0",1)));
                    shell_call_in_progress=1;
                    /*
                        freopen("stdout.buf", "w", stdout);
                        freopen("stderr.buf", "w", stderr);
                    */
                    return_code=system((char*)strz->chr);
                    /*
                        freopen("CON", "w", stdout);
                        freopen("CON", "w", stderr);
                        static char buf[1024];
                        static int buflen;
                        static int fd;
                        fd = open("stdout.buf", O_RDONLY);
                        while((buflen = read(fd, buf, 1024)) > 0)
                        {
                        write(1, buf, buflen);
                        }
                        close(fd);
                        fd = open("stderr.buf", O_RDONLY);
                        while((buflen = read(fd, buf, 1024)) > 0)
                        {
                        write(1, buf, buflen);
                        }
                        close(fd);
                        remove("stdout.buf");
                        remove("stderr.buf");
                    */
                    shell_call_in_progress=0;
                    goto shell_complete;
                }
                
                static STARTUPINFO si;
                static PROCESS_INFORMATION pi;
                
                if (cmd_ok()){
                    
                    static SHELLEXECUTEINFO shi;
                    static char cmd[10]="cmd\0";
                    
                    //attempt to separate file name (if any) from parameters
                    static int32 x,quotes;
                    
                    qbs_set(str1,str);
                    qbs_set(str2,qbs_new_txt(""));
                    if (!str1->len) goto shell_complete;//failed!
                    
                    if (!cmd_command(str1)){
                        //try directly, as is
                        qbs_set(str1z,qbs_add(str1,qbs_new_txt_len("\0",1)));
                        ZeroMemory( &shi, sizeof(shi) );
                        shi.cbSize = sizeof(shi);
                        shi.lpFile=(char*)&str1z->chr[0];
                        shi.lpParameters=NULL;
                        shi.fMask=SEE_MASK_NOCLOSEPROCESS|SEE_MASK_FLAG_NO_UI;
                        shi.nShow=SW_SHOW;
                        if(ShellExecuteEx(&shi)){
                            shell_call_in_progress=1;
                            // Wait until child process exits.
                            WaitForSingleObject( shi.hProcess, INFINITE );
                            GetExitCodeProcess(shi.hProcess,(DWORD*)&return_code);
                            CloseHandle(shi.hProcess);
                            shell_call_in_progress=0;
                            goto shell_complete;
                        }
                    }
                    
                    x=0;
                    quotes=0;
                    while (x<str1->len){
                        if (str1->chr[x]==34){
                            if (!quotes) quotes=1; else quotes=0;
                        }
                        if (str1->chr[x]==32){
                            if (quotes==0){
                                qbs_set(str2,qbs_right(str1,str1->len-x-1));
                                qbs_set(str1,qbs_left(str1,x));
                                goto split;
                            }
                        }
                        x++;
                    }
                    split:
                    if (!str1->len) goto shell_complete;//failed!
                    
                    if (str2->len){ if (!cmd_command(str1)){
                        qbs_set(str1z,qbs_add(str1,qbs_new_txt_len("\0",1)));
                        qbs_set(str2z,qbs_add(str2,qbs_new_txt_len("\0",1)));
                        ZeroMemory( &shi, sizeof(shi) );
                        shi.cbSize = sizeof(shi);
                        shi.lpFile=(char*)&str1z->chr[0];
                        shi.lpParameters=(char*)&str2z->chr[0];
                        //if (str2->len<=1) shi.lpParameters=NULL;
                        shi.fMask=SEE_MASK_NOCLOSEPROCESS|SEE_MASK_FLAG_NO_UI;
                        shi.nShow=SW_SHOW;
                        if(ShellExecuteEx(&shi)){
                            shell_call_in_progress=1;
                            // Wait until child process exits.
                            WaitForSingleObject( shi.hProcess, INFINITE );
                            GetExitCodeProcess(shi.hProcess,(DWORD*)&return_code);
                            CloseHandle(shi.hProcess);
                            shell_call_in_progress=0;
                            goto shell_complete;
                        }
                    }}
                    
                    //failed, try cmd /c method...
                    if (str2->len) qbs_set(str2,qbs_add(qbs_new_txt(" "),str2));
                    qbs_set(strz,qbs_add(str1,str2));
                    qbs_set(strz,qbs_add(qbs_new_txt(" /c "),strz));
                    qbs_set(strz,qbs_add(strz,qbs_new_txt_len("\0",1)));
                    ZeroMemory( &shi, sizeof(shi) );
                    shi.cbSize = sizeof(shi);
                    shi.lpFile=&cmd[0];
                    shi.lpParameters=(char*)&strz->chr[0];
                    shi.fMask=SEE_MASK_NOCLOSEPROCESS|SEE_MASK_FLAG_NO_UI;
                    shi.nShow=SW_SHOW;
                    if(ShellExecuteEx(&shi)){
                        shell_call_in_progress=1;
                        // Wait until child process exits.
                        WaitForSingleObject( shi.hProcess, INFINITE );
                        GetExitCodeProcess(shi.hProcess,(DWORD*)&return_code);
                        CloseHandle(shi.hProcess);
                        shell_call_in_progress=0;
                        goto shell_complete;
                    }
                    
                    /*
                        qbs_set(strz,qbs_add(qbs_new_txt("cmd.exe /c "),str));
                        qbs_set(strz,qbs_add(strz,qbs_new_txt_len("\0",1)));
                        ZeroMemory( &si, sizeof(si) ); si.cb = sizeof(si); ZeroMemory( &pi, sizeof(pi) );
                        if(CreateProcess(
                        NULL,           // No module name (use command line)
                        (char*)&strz->chr[0], // Command line
                        NULL,           // Process handle not inheritable
                        NULL,           // Thread handle not inheritable
                        FALSE,          // Set handle inheritance to FALSE
                        DETACHED_PROCESS, // No creation flags
                        NULL,           // Use parent's environment block
                        NULL,           // Use parent's starting directory 
                        &si,            // Pointer to STARTUPINFO structure
                        &pi )           // Pointer to PROCESS_INFORMATION structure
                        ){
                        shell_call_in_progress=1;
                        // Wait until child process exits.
                        WaitForSingleObject( pi.hProcess, INFINITE );
                        // Close process and thread handles. 
                        CloseHandle( pi.hProcess );
                        CloseHandle( pi.hThread );
                        shell_call_in_progress=0;
                        goto shell_complete;
                        }
                    */
                    
                    return_code=1;
                    goto shell_complete;//failed
                    
                    }else{
                    
                    qbs_set(strz,qbs_add(qbs_new_txt("command.com /c "),str));
                    qbs_set(strz,qbs_add(strz,qbs_new_txt_len("\0",1)));
                    ZeroMemory( &si, sizeof(si) ); si.cb = sizeof(si); ZeroMemory( &pi, sizeof(pi) );
                    if(CreateProcess(
                    NULL,           // No module name (use command line)
                    (char*)&strz->chr[0], // Command line
                    NULL,           // Process handle not inheritable
                    NULL,           // Thread handle not inheritable
                    FALSE,          // Set handle inheritance to FALSE
                    CREATE_NEW_CONSOLE, // No creation flags
                    NULL,           // Use parent's environment block
                    NULL,           // Use parent's starting directory 
                    &si,            // Pointer to STARTUPINFO structure
                    &pi )           // Pointer to PROCESS_INFORMATION structure
                    ){
                        shell_call_in_progress=1;
                        // Wait until child process exits.
                        WaitForSingleObject( pi.hProcess, INFINITE );
                        // Close process and thread handles. 
                        CloseHandle( pi.hProcess );
                        CloseHandle( pi.hThread );
                        shell_call_in_progress=0;
                        goto shell_complete;
                    }
                    goto shell_complete;//failed
                    
                }//cmd_ok()
                
                #else
                
                qbs_set(strz,qbs_add(str,qbs_new_txt_len("\0",1)));
                shell_call_in_progress=1;
                return_code = system((char*)strz->chr);
                shell_call_in_progress=0;
                if (return_code == -1) {/* do nothing */}
                else {
                    return_code = WEXITSTATUS(return_code);
                }
                
            #endif
            
            }else{
            
            //SHELL (with no parameters)
            //note: opening a new shell is only available in windows atm via cmd
            //note: assumes cmd available
            #ifdef QBNex_WINDOWS
                if (!use_console) AllocConsole();
                qbs_set(strz,qbs_new_txt_len("cmd\0",4));
                shell_call_in_progress=1;
                return_code = system((char*)strz->chr);
                shell_call_in_progress=0;
                if (!use_console) FreeConsole();
                goto shell_complete;
            #endif
            
        }
        
        shell_complete:
        //reenter full screen mode if necessary
        if (full_screen_mode){
            full_screen_set=full_screen_mode;
            do{
                Sleep(0);
            }while(!full_screen);
        }//full_screen_mode
        
        return return_code;
    }//func SHELL(...
    
    
    
    
    
    int64 func__shellhide(qbs *str){ //func _SHELLHIDE(...
        if (new_error) return 1;
        
        static int64 return_code;
        return_code=0;
        
        static qbs *strz=NULL;
        static int32 i;
        if (!strz) strz=qbs_new(0,0);
        if (!str->len){error(5); return 1;}//cannot launch a hidden console
        
        static qbs *str1=NULL;
        static qbs *str2=NULL;
        static qbs *str1z=NULL;
        static qbs *str2z=NULL;
        if (!str1) str1=qbs_new(0,0);
        if (!str2) str2=qbs_new(0,0);
        if (!str1z) str1z=qbs_new(0,0);
        if (!str2z) str2z=qbs_new(0,0);
        
        #ifdef QBNex_WINDOWS
            
            static STARTUPINFO si;
            static PROCESS_INFORMATION pi;
            
            if (cmd_ok()){
                
                static SHELLEXECUTEINFO shi;
                static char cmd[10]="cmd\0";
                
                //attempt to separate file name (if any) from parameters
                static int32 x,quotes;
                
                qbs_set(str1,str);
                qbs_set(str2,qbs_new_txt(""));
                
                if (!cmd_command(str1)){
                    //try directly, as is
                    qbs_set(str1z,qbs_add(str1,qbs_new_txt_len("\0",1)));
                    ZeroMemory( &shi, sizeof(shi) );
                    shi.cbSize = sizeof(shi);
                    shi.lpFile=(char*)&str1z->chr[0];
                    shi.lpParameters=NULL;
                    shi.fMask=SEE_MASK_NOCLOSEPROCESS|SEE_MASK_FLAG_NO_UI;
                    shi.nShow=SW_HIDE;
                    if(ShellExecuteEx(&shi)){
                        shell_call_in_progress=1;
                        // Wait until child process exits.
                        WaitForSingleObject( shi.hProcess, INFINITE );
                        GetExitCodeProcess(shi.hProcess,(DWORD*)&return_code);
                        CloseHandle(shi.hProcess);
                        shell_call_in_progress=0;
                        goto shell_complete;
                    }
                }
                
                x=0;
                quotes=0;
                while (x<str1->len){
                    if (str1->chr[x]==34){
                        if (!quotes) quotes=1; else quotes=0;
                    }
                    if (str1->chr[x]==32){
                        if (quotes==0){
                            qbs_set(str2,qbs_right(str1,str1->len-x-1));
                            qbs_set(str1,qbs_left(str1,x));
                            goto split;
                        }
                    }
                    x++;
                }
                split:
                if (!str1->len) {return_code=1; goto shell_complete;}//failed!
                
                if (str2->len){ if (!cmd_command(str1)){
                    qbs_set(str1z,qbs_add(str1,qbs_new_txt_len("\0",1)));
                    qbs_set(str2z,qbs_add(str2,qbs_new_txt_len("\0",1)));
                    ZeroMemory( &shi, sizeof(shi) );
                    shi.cbSize = sizeof(shi);
                    shi.lpFile=(char*)&str1z->chr[0];
                    shi.lpParameters=(char*)&str2z->chr[0];
                    //if (str2->len<=1) shi.lpParameters=NULL;
                    shi.fMask=SEE_MASK_NOCLOSEPROCESS|SEE_MASK_FLAG_NO_UI;
                    shi.nShow=SW_HIDE;
                    if(ShellExecuteEx(&shi)){
                        shell_call_in_progress=1;
                        // Wait until child process exits.
                        WaitForSingleObject( shi.hProcess, INFINITE );
                        GetExitCodeProcess(shi.hProcess,(DWORD*)&return_code);
                        CloseHandle(shi.hProcess);
                        shell_call_in_progress=0;
                        goto shell_complete;
                    }
                }}
                
                //failed, try cmd /c method...
                if (str2->len) qbs_set(str2,qbs_add(qbs_new_txt(" "),str2));
                qbs_set(strz,qbs_add(str1,str2));
                qbs_set(strz,qbs_add(qbs_new_txt(" /c "),strz));
                qbs_set(strz,qbs_add(strz,qbs_new_txt_len("\0",1)));
                ZeroMemory( &shi, sizeof(shi) );
                shi.cbSize = sizeof(shi);
                shi.lpFile=&cmd[0];
                shi.lpParameters=(char*)&strz->chr[0];
                shi.fMask=SEE_MASK_NOCLOSEPROCESS|SEE_MASK_FLAG_NO_UI;
                shi.nShow=SW_HIDE;
                if(ShellExecuteEx(&shi)){
                    shell_call_in_progress=1;
                    // Wait until child process exits.
                    WaitForSingleObject( shi.hProcess, INFINITE );
                    GetExitCodeProcess(shi.hProcess,(DWORD*)&return_code);
                    CloseHandle(shi.hProcess);
                    shell_call_in_progress=0;
                    goto shell_complete;
                }
                
                /*
                    qbs_set(strz,qbs_add(qbs_new_txt("cmd.exe /c "),str));
                    qbs_set(strz,qbs_add(strz,qbs_new_txt_len("\0",1)));
                    ZeroMemory( &si, sizeof(si) ); si.cb = sizeof(si); ZeroMemory( &pi, sizeof(pi) );
                    if(CreateProcess(
                    NULL,           // No module name (use command line)
                    (char*)&strz->chr[0], // Command line
                    NULL,           // Process handle not inheritable
                    NULL,           // Thread handle not inheritable
                    FALSE,          // Set handle inheritance to FALSE
                    CREATE_NO_WINDOW, // No creation flags
                    NULL,           // Use parent's environment block
                    NULL,           // Use parent's starting directory 
                    &si,            // Pointer to STARTUPINFO structure
                    &pi )           // Pointer to PROCESS_INFORMATION structure
                    ){
                    shell_call_in_progress=1;
                    // Wait until child process exits.
                    WaitForSingleObject( pi.hProcess, INFINITE );
                    // Close process and thread handles. 
                    CloseHandle( pi.hProcess );
                    CloseHandle( pi.hThread );
                    shell_call_in_progress=0;
                    goto shell_complete;
                    }
                */
                
                return_code=1;
                goto shell_complete;//failed
                
                }else{
                
                qbs_set(strz,qbs_add(qbs_new_txt("command.com /c "),str));
                qbs_set(strz,qbs_add(strz,qbs_new_txt_len("\0",1)));
                ZeroMemory( &si, sizeof(si) ); si.cb = sizeof(si); ZeroMemory( &pi, sizeof(pi) );
                if(CreateProcess(
                
                NULL,           // No module name (use command line)
                (char*)&strz->chr[0], // Command line
                NULL,           // Process handle not inheritable
                NULL,           // Thread handle not inheritable
                FALSE,          // Set handle inheritance to FALSE
                CREATE_NEW_CONSOLE, //note: cannot hide new console, but can preserve existing one
                NULL,           // Use parent's environment block
                NULL,           // Use parent's starting directory 
                &si,            // Pointer to STARTUPINFO structure
                &pi )           // Pointer to PROCESS_INFORMATION structure
                ){
                    shell_call_in_progress=1;
                    // Wait until child process exits.
                    WaitForSingleObject( pi.hProcess, INFINITE );
                    // Close process and thread handles. 
                    CloseHandle( pi.hProcess );
                    CloseHandle( pi.hThread );
                    shell_call_in_progress=0;
                    goto shell_complete;
                }
                goto shell_complete;//failed
                
            }//cmd_ok()
            
            #else
            
            qbs_set(strz,qbs_add(str,qbs_new_txt_len("\0",1)));
            shell_call_in_progress=1;
            return_code=system((char*)strz->chr);
            shell_call_in_progress=0;
            return return_code;
            
        #endif
        
        shell_complete:;
        
        return return_code;
    }//func _SHELLHIDE(...
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    void sub_shell(qbs *str,int32 passed){
        if (new_error) return;
        
        //exit full screen mode if necessary
        static int32 full_screen_mode;
        full_screen_mode=full_screen;
        if (full_screen_mode){
            full_screen_set=0;
            do{
                Sleep(0);
            }while(full_screen);
        }//full_screen_mode
        static qbs *strz=NULL;
        static qbs *str1=NULL;
        static qbs *str1z=NULL;
        static qbs *str2=NULL;
        static qbs *str2z=NULL;
        static int32 i;
        
        static int32 use_console;
        use_console=0;
        if (console){
            if (console_active){
                use_console=1;
            }
        }
        
        if (!strz) strz=qbs_new(0,0);
        if (!str1) str1=qbs_new(0,0);
        if (!str1z) str1z=qbs_new(0,0);
        if (!str2) str2=qbs_new(0,0);
        if (!str2z) str2z=qbs_new(0,0);
        
        if (passed) if (str->len==0) passed=0;//"" means launch a CONSOLE
        if (passed){
            
            
            
            #ifdef QBNex_WINDOWS
                
                
                if (use_console){
                    qbs_set(strz,qbs_add(str,qbs_new_txt_len("\0",1)));
                    shell_call_in_progress=1;
                    /*
                        freopen("stdout.buf", "w", stdout);
                        freopen("stderr.buf", "w", stderr);
                    */
                    system((char*)strz->chr);
                    /*
                        freopen("CON", "w", stdout);
                        freopen("CON", "w", stderr);
                        static char buf[1024];
                        static int buflen;
                        static int fd;
                        fd = open("stdout.buf", O_RDONLY);
                        while((buflen = read(fd, buf, 1024)) > 0)
                        {
                        write(1, buf, buflen);
                        }
                        close(fd);
                        fd = open("stderr.buf", O_RDONLY);
                        while((buflen = read(fd, buf, 1024)) > 0)
                        {
                        write(1, buf, buflen);
                        }
                        close(fd);
                        remove("stdout.buf");
                        remove("stderr.buf");
                    */
                    shell_call_in_progress=0;
                    goto shell_complete;
                }
                
                
                
                static STARTUPINFO si;
                static PROCESS_INFORMATION pi;
                
                if (cmd_ok()){
                    
                    static SHELLEXECUTEINFO shi;
                    static char cmd[10]="cmd\0";
                    
                    //attempt to separate file name (if any) from parameters
                    static int32 x,quotes;
                    
                    qbs_set(str1,str);
                    qbs_set(str2,qbs_new_txt(""));
                    if (!str1->len) goto shell_complete;//failed!
                    
                    if (!cmd_command(str1)){
                        //try directly, as is
                        qbs_set(str1z,qbs_add(str1,qbs_new_txt_len("\0",1)));
                        ZeroMemory( &shi, sizeof(shi) );
                        shi.cbSize = sizeof(shi);
                        shi.lpFile=(char*)&str1z->chr[0];
                        shi.lpParameters=NULL;
                        shi.fMask=SEE_MASK_NOCLOSEPROCESS|SEE_MASK_FLAG_NO_UI;
                        shi.nShow=SW_SHOW;
                        if(ShellExecuteEx(&shi)){
                            shell_call_in_progress=1;
                            // Wait until child process exits.
                            WaitForSingleObject( shi.hProcess, INFINITE );
                            CloseHandle(shi.hProcess);
                            shell_call_in_progress=0;
                            goto shell_complete;
                        }
                    }
                    
                    x=0;
                    quotes=0;
                    while (x<str1->len){
                        if (str1->chr[x]==34){
                            if (!quotes) quotes=1; else quotes=0;
                        }
                        if (str1->chr[x]==32){
                            if (quotes==0){
                                qbs_set(str2,qbs_right(str1,str1->len-x-1));
                                qbs_set(str1,qbs_left(str1,x));
                                goto split;
                            }
                        }
                        x++;
                    }
                    split:
                    if (!str1->len) goto shell_complete;//failed!
                    
                    if (str2->len){ if (!cmd_command(str1)){
                        qbs_set(str1z,qbs_add(str1,qbs_new_txt_len("\0",1)));
                        qbs_set(str2z,qbs_add(str2,qbs_new_txt_len("\0",1)));
                        ZeroMemory( &shi, sizeof(shi) );
                        shi.cbSize = sizeof(shi);
                        shi.lpFile=(char*)&str1z->chr[0];
                        shi.lpParameters=(char*)&str2z->chr[0];
                        //if (str2->len<=1) shi.lpParameters=NULL;
                        shi.fMask=SEE_MASK_NOCLOSEPROCESS|SEE_MASK_FLAG_NO_UI;
                        shi.nShow=SW_SHOW;
                        if(ShellExecuteEx(&shi)){
                            shell_call_in_progress=1;
                            // Wait until child process exits.
                            WaitForSingleObject( shi.hProcess, INFINITE );
                            CloseHandle(shi.hProcess);
                            shell_call_in_progress=0;
                            goto shell_complete;
                        }
                    }}
                    
                    //failed, try cmd /c method...
                    if (str2->len) qbs_set(str2,qbs_add(qbs_new_txt(" "),str2));
                    qbs_set(strz,qbs_add(str1,str2));
                    qbs_set(strz,qbs_add(qbs_new_txt(" /c "),strz));
                    qbs_set(strz,qbs_add(strz,qbs_new_txt_len("\0",1)));
                    ZeroMemory( &shi, sizeof(shi) );
                    shi.cbSize = sizeof(shi);
                    shi.lpFile=&cmd[0];
                    shi.lpParameters=(char*)&strz->chr[0];
                    shi.fMask=SEE_MASK_NOCLOSEPROCESS|SEE_MASK_FLAG_NO_UI;
                    shi.nShow=SW_SHOW;
                    if(ShellExecuteEx(&shi)){
                        shell_call_in_progress=1;
                        // Wait until child process exits.
                        WaitForSingleObject( shi.hProcess, INFINITE );
                        CloseHandle(shi.hProcess);
                        shell_call_in_progress=0;
                        goto shell_complete;
                    }
                    
                    /*
                        qbs_set(strz,qbs_add(qbs_new_txt("cmd.exe /c "),str));
                        qbs_set(strz,qbs_add(strz,qbs_new_txt_len("\0",1)));
                        ZeroMemory( &si, sizeof(si) ); si.cb = sizeof(si); ZeroMemory( &pi, sizeof(pi) );
                        if(CreateProcess(
                        NULL,           // No module name (use command line)
                        (char*)&strz->chr[0], // Command line
                        NULL,           // Process handle not inheritable
                        NULL,           // Thread handle not inheritable
                        FALSE,          // Set handle inheritance to FALSE
                        DETACHED_PROCESS, // No creation flags
                        NULL,           // Use parent's environment block
                        NULL,           // Use parent's starting directory 
                        &si,            // Pointer to STARTUPINFO structure
                        &pi )           // Pointer to PROCESS_INFORMATION structure
                        ){
                        shell_call_in_progress=1;
                        // Wait until child process exits.
                        WaitForSingleObject( pi.hProcess, INFINITE );
                        // Close process and thread handles. 
                        CloseHandle( pi.hProcess );
                        CloseHandle( pi.hThread );
                        shell_call_in_progress=0;
                        goto shell_complete;
                        }
                    */
                    
                    goto shell_complete;//failed
                    
                    }else{
                    
                    qbs_set(strz,qbs_add(qbs_new_txt("command.com /c "),str));
                    qbs_set(strz,qbs_add(strz,qbs_new_txt_len("\0",1)));
                    ZeroMemory( &si, sizeof(si) ); si.cb = sizeof(si); ZeroMemory( &pi, sizeof(pi) );
                    if(CreateProcess(
                    NULL,           // No module name (use command line)
                    (char*)&strz->chr[0], // Command line
                    NULL,           // Process handle not inheritable
                    NULL,           // Thread handle not inheritable
                    FALSE,          // Set handle inheritance to FALSE
                    CREATE_NEW_CONSOLE, // No creation flags
                    NULL,           // Use parent's environment block
                    NULL,           // Use parent's starting directory 
                    &si,            // Pointer to STARTUPINFO structure
                    &pi )           // Pointer to PROCESS_INFORMATION structure
                    ){
                        shell_call_in_progress=1;
                        // Wait until child process exits.
                        WaitForSingleObject( pi.hProcess, INFINITE );
                        // Close process and thread handles. 
                        CloseHandle( pi.hProcess );
                        CloseHandle( pi.hThread );
                        shell_call_in_progress=0;
                        goto shell_complete;
                    }
                    goto shell_complete;//failed
                    
                }//cmd_ok()
                
                #else
                
                qbs_set(strz,qbs_add(str,qbs_new_txt_len("\0",1)));
                shell_call_in_progress=1;
                system((char*)strz->chr);
                shell_call_in_progress=0;
                
            #endif
            
            }else{
            
            //SHELL (with no parameters)
            //note: opening a new shell is only available in windows atm via cmd
            //note: assumes cmd available
            #ifdef QBNex_WINDOWS
                if (!use_console) AllocConsole();
                qbs_set(strz,qbs_new_txt_len("cmd\0",4));
                shell_call_in_progress=1;
                system((char*)strz->chr);
                shell_call_in_progress=0;
                if (!use_console) FreeConsole();
                goto shell_complete;
            #endif
            
        }
        
        shell_complete:
        //reenter full screen mode if necessary
        if (full_screen_mode){
            full_screen_set=full_screen_mode;
            do{
                Sleep(0);
            }while(!full_screen);
        }//full_screen_mode
    }
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    void sub_shell2(qbs *str,int32 passed){ //HIDE
        if (new_error) return;
        
        if (passed&1){sub_shell4(str,passed&2); return;}
        if (!(passed&2)){error(5); return;}//should not hide a shell waiting for input
        
        static qbs *strz=NULL;
        static int32 i;
        if (!strz) strz=qbs_new(0,0);
        if (!str->len){error(5); return;}//cannot launch a hidden console
        
        static qbs *str1=NULL;
        static qbs *str2=NULL;
        static qbs *str1z=NULL;
        static qbs *str2z=NULL;
        if (!str1) str1=qbs_new(0,0);
        if (!str2) str2=qbs_new(0,0);
        if (!str1z) str1z=qbs_new(0,0);
        if (!str2z) str2z=qbs_new(0,0);
        
        #ifdef QBNex_WINDOWS
            
            static STARTUPINFO si;
            static PROCESS_INFORMATION pi;
            
            if (cmd_ok()){
                
                static SHELLEXECUTEINFO shi;
                static char cmd[10]="cmd\0";
                
                //attempt to separate file name (if any) from parameters
                static int32 x,quotes;
                
                qbs_set(str1,str);
                qbs_set(str2,qbs_new_txt(""));
                
                if (!cmd_command(str1)){
                    //try directly, as is
                    qbs_set(str1z,qbs_add(str1,qbs_new_txt_len("\0",1)));
                    ZeroMemory( &shi, sizeof(shi) );
                    shi.cbSize = sizeof(shi);
                    shi.lpFile=(char*)&str1z->chr[0];
                    shi.lpParameters=NULL;
                    shi.fMask=SEE_MASK_NOCLOSEPROCESS|SEE_MASK_FLAG_NO_UI;
                    shi.nShow=SW_HIDE;
                    if(ShellExecuteEx(&shi)){
                        shell_call_in_progress=1;
                        // Wait until child process exits.
                        WaitForSingleObject( shi.hProcess, INFINITE );
                        CloseHandle(shi.hProcess);
                        shell_call_in_progress=0;
                        goto shell_complete;
                    }
                }
                
                x=0;
                quotes=0;
                while (x<str1->len){
                    if (str1->chr[x]==34){
                        if (!quotes) quotes=1; else quotes=0;
                    }
                    if (str1->chr[x]==32){
                        if (quotes==0){
                            qbs_set(str2,qbs_right(str1,str1->len-x-1));
                            qbs_set(str1,qbs_left(str1,x));
                            goto split;
                        }
                    }
                    x++;
                }
                split:
                if (!str1->len) goto shell_complete;//failed!
                
                if (str2->len){ if (!cmd_command(str1)){
                    qbs_set(str1z,qbs_add(str1,qbs_new_txt_len("\0",1)));
                    qbs_set(str2z,qbs_add(str2,qbs_new_txt_len("\0",1)));
                    ZeroMemory( &shi, sizeof(shi) );
                    shi.cbSize = sizeof(shi);
                    shi.lpFile=(char*)&str1z->chr[0];
                    shi.lpParameters=(char*)&str2z->chr[0];
                    //if (str2->len<=1) shi.lpParameters=NULL;
                    shi.fMask=SEE_MASK_NOCLOSEPROCESS|SEE_MASK_FLAG_NO_UI;
                    shi.nShow=SW_HIDE;
                    if(ShellExecuteEx(&shi)){
                        shell_call_in_progress=1;
                        // Wait until child process exits.
                        WaitForSingleObject( shi.hProcess, INFINITE );
                        CloseHandle(shi.hProcess);
                        shell_call_in_progress=0;
                        goto shell_complete;
                    }
                }}
                
                //failed, try cmd /c method...
                if (str2->len) qbs_set(str2,qbs_add(qbs_new_txt(" "),str2));
                qbs_set(strz,qbs_add(str1,str2));
                qbs_set(strz,qbs_add(qbs_new_txt(" /c "),strz));
                qbs_set(strz,qbs_add(strz,qbs_new_txt_len("\0",1)));
                ZeroMemory( &shi, sizeof(shi) );
                shi.cbSize = sizeof(shi);
                shi.lpFile=&cmd[0];
                shi.lpParameters=(char*)&strz->chr[0];
                shi.fMask=SEE_MASK_NOCLOSEPROCESS|SEE_MASK_FLAG_NO_UI;
                shi.nShow=SW_HIDE;
                if(ShellExecuteEx(&shi)){
                    shell_call_in_progress=1;
                    // Wait until child process exits.
                    WaitForSingleObject( shi.hProcess, INFINITE );
                    CloseHandle(shi.hProcess);
                    shell_call_in_progress=0;
                    goto shell_complete;
                }
                
                /*
                    qbs_set(strz,qbs_add(qbs_new_txt("cmd.exe /c "),str));
                    qbs_set(strz,qbs_add(strz,qbs_new_txt_len("\0",1)));
                    ZeroMemory( &si, sizeof(si) ); si.cb = sizeof(si); ZeroMemory( &pi, sizeof(pi) );
                    if(CreateProcess(
                    NULL,           // No module name (use command line)
                    (char*)&strz->chr[0], // Command line
                    NULL,           // Process handle not inheritable
                    NULL,           // Thread handle not inheritable
                    FALSE,          // Set handle inheritance to FALSE
                    CREATE_NO_WINDOW, // No creation flags
                    NULL,           // Use parent's environment block
                    NULL,           // Use parent's starting directory 
                    &si,            // Pointer to STARTUPINFO structure
                    &pi )           // Pointer to PROCESS_INFORMATION structure
                    ){
                    shell_call_in_progress=1;
                    // Wait until child process exits.
                    WaitForSingleObject( pi.hProcess, INFINITE );
                    // Close process and thread handles. 
                    CloseHandle( pi.hProcess );
                    CloseHandle( pi.hThread );
                    shell_call_in_progress=0;
                    goto shell_complete;
                    }
                */
                
                goto shell_complete;//failed
                
                }else{
                
                qbs_set(strz,qbs_add(qbs_new_txt("command.com /c "),str));
                qbs_set(strz,qbs_add(strz,qbs_new_txt_len("\0",1)));
                ZeroMemory( &si, sizeof(si) ); si.cb = sizeof(si); ZeroMemory( &pi, sizeof(pi) );
                if(CreateProcess(
                
                NULL,           // No module name (use command line)
                (char*)&strz->chr[0], // Command line
                NULL,           // Process handle not inheritable
                NULL,           // Thread handle not inheritable
                FALSE,          // Set handle inheritance to FALSE
                CREATE_NEW_CONSOLE, //note: cannot hide new console, but can preserve existing one
                NULL,           // Use parent's environment block
                NULL,           // Use parent's starting directory 
                &si,            // Pointer to STARTUPINFO structure
                &pi )           // Pointer to PROCESS_INFORMATION structure
                ){
                    shell_call_in_progress=1;
                    // Wait until child process exits.
                    WaitForSingleObject( pi.hProcess, INFINITE );
                    // Close process and thread handles. 
                    CloseHandle( pi.hProcess );
                    CloseHandle( pi.hThread );
                    shell_call_in_progress=0;
                    goto shell_complete;
                }
                goto shell_complete;//failed
                
            }//cmd_ok()
            
            #else
            
            qbs_set(strz,qbs_add(str,qbs_new_txt_len("\0",1)));
            shell_call_in_progress=1;
            system((char*)strz->chr);
            shell_call_in_progress=0;
            return;
            
        #endif
        
        shell_complete:;
    }
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    void sub_shell3(qbs *str,int32 passed){//_DONTWAIT
        //shell3 launches 'str' but does not wait for it to complete
        if (new_error) return;
        
        if (passed&1){sub_shell4(str,passed&2); return;}
        
        static qbs *strz=NULL;
        static int32 i;
        
        static qbs *str1=NULL;
        static qbs *str2=NULL;
        static qbs *str1z=NULL;
        static qbs *str2z=NULL;
        if (!str1) str1=qbs_new(0,0);
        if (!str2) str2=qbs_new(0,0);
        if (!str1z) str1z=qbs_new(0,0);
        if (!str2z) str2z=qbs_new(0,0);
        
        if (!strz) strz=qbs_new(0,0);
        
        #ifdef QBNex_WINDOWS
            
            static STARTUPINFO si;
            static PROCESS_INFORMATION pi;
            
            if (cmd_ok()){
                
                static SHELLEXECUTEINFO shi;
                static char cmd[10]="cmd\0";
                
                //attempt to separate file name (if any) from parameters
                static int32 x,quotes;
                
                //allow for launching of a console
                if (!(passed&2)){
                    qbs_set(str1,qbs_new_txt("cmd"));
                    }else{
                    qbs_set(str1,str);
                    if (!str1->len) qbs_set(str1,qbs_new_txt("cmd"));
                }
                qbs_set(str2,qbs_new_txt(""));
                
                if (!cmd_command(str1)){
                    //try directly, as is
                    qbs_set(str1z,qbs_add(str1,qbs_new_txt_len("\0",1)));
                    ZeroMemory( &shi, sizeof(shi) );
                    shi.cbSize = sizeof(shi);
                    shi.lpFile=(char*)&str1z->chr[0];
                    shi.lpParameters=NULL;
                    shi.fMask=SEE_MASK_FLAG_NO_UI;
                    shi.nShow=SW_SHOW;
                    if(ShellExecuteEx(&shi)){
                        goto shell_complete;
                    }
                }
                
                x=0;
                quotes=0;
                while (x<str1->len){
                    if (str1->chr[x]==34){
                        if (!quotes) quotes=1; else quotes=0;
                    }
                    if (str1->chr[x]==32){
                        if (quotes==0){
                            qbs_set(str2,qbs_right(str1,str1->len-x-1));
                            qbs_set(str1,qbs_left(str1,x));
                            goto split;
                        }
                    }
                    x++;
                }
                split:
                if (!str1->len) goto shell_complete;//failed!
                
                if (str2->len){ if (!cmd_command(str1)){
                    qbs_set(str1z,qbs_add(str1,qbs_new_txt_len("\0",1)));
                    qbs_set(str2z,qbs_add(str2,qbs_new_txt_len("\0",1)));
                    ZeroMemory( &shi, sizeof(shi) );
                    shi.cbSize = sizeof(shi);
                    shi.lpFile=(char*)&str1z->chr[0];
                    shi.lpParameters=(char*)&str2z->chr[0];
                    //if (str2->len<=1) shi.lpParameters=NULL;
                    shi.fMask=SEE_MASK_FLAG_NO_UI;
                    shi.nShow=SW_SHOW;
                    if(ShellExecuteEx(&shi)){
                        goto shell_complete;
                    }
                }}
                
                //failed, try cmd /c method...
                if (str2->len) qbs_set(str2,qbs_add(qbs_new_txt(" "),str2));
                qbs_set(strz,qbs_add(str1,str2));
                qbs_set(strz,qbs_add(qbs_new_txt(" /c "),strz));
                qbs_set(strz,qbs_add(strz,qbs_new_txt_len("\0",1)));
                ZeroMemory( &shi, sizeof(shi) );
                shi.cbSize = sizeof(shi);
                shi.lpFile=&cmd[0];
                shi.lpParameters=(char*)&strz->chr[0];
                shi.fMask=SEE_MASK_FLAG_NO_UI;
                shi.nShow=SW_SHOW;
                if(ShellExecuteEx(&shi)){
                    goto shell_complete;
                }
                
                /*
                    qbs_set(strz,qbs_add(qbs_new_txt("cmd.exe /c "),str));
                    qbs_set(strz,qbs_add(strz,qbs_new_txt_len("\0",1)));
                    ZeroMemory( &si, sizeof(si) ); si.cb = sizeof(si); ZeroMemory( &pi, sizeof(pi) );
                    if(CreateProcess(
                    NULL,           // No module name (use command line)
                    (char*)&strz->chr[0], // Command line
                    NULL,           // Process handle not inheritable
                    NULL,           // Thread handle not inheritable
                    FALSE,          // Set handle inheritance to FALSE
                    DETACHED_PROCESS, // No creation flags
                    NULL,           // Use parent's environment block
                    NULL,           // Use parent's starting directory 
                    &si,            // Pointer to STARTUPINFO structure
                    &pi )           // Pointer to PROCESS_INFORMATION structure
                    ){
                    //ref: The created process remains in the system until all threads within the process have terminated and all handles to the process and any of its threads have been closed through calls to CloseHandle. The handles for both the process and the main thread must be closed through calls to CloseHandle. If these handles are not needed, it is best to close them immediately after the process is created. 
                    CloseHandle( pi.hProcess );
                    CloseHandle( pi.hThread );
                    goto shell_complete;
                    }
                */
                
                goto shell_complete;//failed
                
                }else{
                
                qbs_set(strz,qbs_add(qbs_new_txt("command.com /c "),str));
                qbs_set(strz,qbs_add(strz,qbs_new_txt_len("\0",1)));
                ZeroMemory( &si, sizeof(si) ); si.cb = sizeof(si); ZeroMemory( &pi, sizeof(pi) );
                if(CreateProcess(
                NULL,           // No module name (use command line)
                (char*)&strz->chr[0], // Command line
                NULL,           // Process handle not inheritable
                NULL,           // Thread handle not inheritable
                FALSE,          // Set handle inheritance to FALSE
                CREATE_NEW_CONSOLE, //note: cannot hide new console, but can preserve existing one
                NULL,           // Use parent's environment block
                NULL,           // Use parent's starting directory 
                &si,            // Pointer to STARTUPINFO structure
                &pi )           // Pointer to PROCESS_INFORMATION structure
                ){
                    //ref: The created process remains in the system until all threads within the process have terminated and all handles to the process and any of its threads have been closed through calls to CloseHandle. The handles for both the process and the main thread must be closed through calls to CloseHandle. If these handles are not needed, it is best to close them immediately after the process is created. 
                    CloseHandle( pi.hProcess );
                    CloseHandle( pi.hThread );
                    goto shell_complete;
                }
                goto shell_complete;//failed
                
            }//cmd_ok()
            
            #else
            
            qbs_set(strz,qbs_add(str,qbs_new_txt_len("\0",1)));
            pid_t pid = fork ();
            if (pid == 0)
            _exit (system((char*)strz->chr));
            return;
            
        #endif
        
        shell_complete:;
    }//SHELL _DONTWAIT
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    void sub_shell4(qbs *str,int32 passed){//_DONTWAIT & _HIDE
        //if passed&2 set a string was given
        if (!(passed&2)){error(5); return;}//should not hide a shell waiting for input
        
        static qbs *strz=NULL;
        static int32 i;
        
        static qbs *str1=NULL;
        static qbs *str2=NULL;
        static qbs *str1z=NULL;
        static qbs *str2z=NULL;
        if (!str1) str1=qbs_new(0,0);
        if (!str2) str2=qbs_new(0,0);
        if (!str1z) str1z=qbs_new(0,0);
        if (!str2z) str2z=qbs_new(0,0);
        
        if (!strz) strz=qbs_new(0,0);
        
        if (!str->len){error(5); return;}//console unsupported
        
        #ifdef QBNex_WINDOWS
            
            static STARTUPINFO si;
            static PROCESS_INFORMATION pi;
            
            if (cmd_ok()){
                
                static SHELLEXECUTEINFO shi;
                static char cmd[10]="cmd\0";
                
                //attempt to separate file name (if any) from parameters
                static int32 x,quotes;
                
                qbs_set(str1,str);
                qbs_set(str2,qbs_new_txt(""));
                
                if (!cmd_command(str1)){
                    //try directly, as is
                    qbs_set(str1z,qbs_add(str1,qbs_new_txt_len("\0",1)));
                    ZeroMemory( &shi, sizeof(shi) );
                    shi.cbSize = sizeof(shi);
                    shi.lpFile=(char*)&str1z->chr[0];
                    shi.lpParameters=NULL;
                    shi.fMask=SEE_MASK_FLAG_NO_UI;
                    shi.nShow=SW_HIDE;
                    if(ShellExecuteEx(&shi)){
                        goto shell_complete;
                    }
                }
                
                x=0;
                quotes=0;
                while (x<str1->len){
                    if (str1->chr[x]==34){
                        if (!quotes) quotes=1; else quotes=0;
                    }
                    if (str1->chr[x]==32){
                        if (quotes==0){
                            qbs_set(str2,qbs_right(str1,str1->len-x-1));
                            qbs_set(str1,qbs_left(str1,x));
                            goto split;
                        }
                    }
                    x++;
                }
                split:
                if (!str1->len) goto shell_complete;//failed!
                
                if (str2->len){ if (!cmd_command(str1)){
                    qbs_set(str1z,qbs_add(str1,qbs_new_txt_len("\0",1)));
                    qbs_set(str2z,qbs_add(str2,qbs_new_txt_len("\0",1)));
                    ZeroMemory( &shi, sizeof(shi) );
                    shi.cbSize = sizeof(shi);
                    shi.lpFile=(char*)&str1z->chr[0];
                    shi.lpParameters=(char*)&str2z->chr[0];
                    //if (str2->len<=1) shi.lpParameters=NULL;
                    shi.fMask=SEE_MASK_FLAG_NO_UI;
                    shi.nShow=SW_HIDE;
                    if(ShellExecuteEx(&shi)){
                        goto shell_complete;
                    }
                }}
                
                //failed, try cmd /c method...
                if (str2->len) qbs_set(str2,qbs_add(qbs_new_txt(" "),str2));
                qbs_set(strz,qbs_add(str1,str2));
                qbs_set(strz,qbs_add(qbs_new_txt(" /c "),strz));
                qbs_set(strz,qbs_add(strz,qbs_new_txt_len("\0",1)));
                ZeroMemory( &shi, sizeof(shi) );
                shi.cbSize = sizeof(shi);
                shi.lpFile=&cmd[0];
                shi.lpParameters=(char*)&strz->chr[0];
                shi.fMask=SEE_MASK_FLAG_NO_UI;
                shi.nShow=SW_HIDE;
                if(ShellExecuteEx(&shi)){
                    goto shell_complete;
                }
                
                /*
                    qbs_set(strz,qbs_add(qbs_new_txt("cmd.exe /c "),str));
                    qbs_set(strz,qbs_add(strz,qbs_new_txt_len("\0",1)));
                    ZeroMemory( &si, sizeof(si) ); si.cb = sizeof(si); ZeroMemory( &pi, sizeof(pi) );
                    if(CreateProcess(
                    NULL,           // No module name (use command line)
                    (char*)&strz->chr[0], // Command line
                    NULL,           // Process handle not inheritable
                    NULL,           // Thread handle not inheritable
                    FALSE,          // Set handle inheritance to FALSE
                    DETACHED_PROCESS, // No creation flags
                    NULL,           // Use parent's environment block
                    NULL,           // Use parent's starting directory 
                    &si,            // Pointer to STARTUPINFO structure
                    &pi )           // Pointer to PROCESS_INFORMATION structure
                    ){
                    //ref: The created process remains in the system until all threads within the process have terminated and all handles to the process and any of its threads have been closed through calls to CloseHandle. The handles for both the process and the main thread must be closed through calls to CloseHandle. If these handles are not needed, it is best to close them immediately after the process is created. 
                    CloseHandle( pi.hProcess );
                    CloseHandle( pi.hThread );
                    goto shell_complete;
                    }
                */
                
                goto shell_complete;//failed
                
                }else{
                
                qbs_set(strz,qbs_add(qbs_new_txt("command.com /c "),str));
                qbs_set(strz,qbs_add(strz,qbs_new_txt_len("\0",1)));
                ZeroMemory( &si, sizeof(si) ); si.cb = sizeof(si); ZeroMemory( &pi, sizeof(pi) );
                if(CreateProcess(
                NULL,           // No module name (use command line)
                (char*)&strz->chr[0], // Command line
                NULL,           // Process handle not inheritable
                NULL,           // Thread handle not inheritable
                FALSE,          // Set handle inheritance to FALSE
                CREATE_NEW_CONSOLE, //note: cannot hide new console, but can preserve existing one
                NULL,           // Use parent's environment block
                NULL,           // Use parent's starting directory 
                &si,            // Pointer to STARTUPINFO structure
                &pi )           // Pointer to PROCESS_INFORMATION structure
                ){
                    //ref: The created process remains in the system until all threads within the process have terminated and all handles to the process and any of its threads have been closed through calls to CloseHandle. The handles for both the process and the main thread must be closed through calls to CloseHandle. If these handles are not needed, it is best to close them immediately after the process is created. 
                    CloseHandle( pi.hProcess );
                    CloseHandle( pi.hThread );
                    goto shell_complete;
                }
                goto shell_complete;//failed
                
            }//cmd_ok()
            
            #else
            
            qbs_set(strz,qbs_add(str,qbs_new_txt_len("\0",1)));
            pid_t pid = fork ();
            if (pid == 0)
            _exit (system((char*)strz->chr));
            return;
            
        #endif
        
        shell_complete:;
        
    }//_DONTWAIT & _HIDE
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    void sub_kill(qbs *str){
        //note: file not found returned for non-existant paths too
        //      file already open returned if access unavailable
        if (new_error) return;
        static int32 i;
        static qbs *strz=NULL;
        if (!strz) strz=qbs_new(0,0);
        qbs_set(strz,qbs_add(str,qbs_new_txt_len("\0",1)));
        #ifdef QBNex_WINDOWS
            static WIN32_FIND_DATA fd;
            static HANDLE hFind;
            static qbs *strpath=NULL; if (!strpath) strpath=qbs_new(0,0);
            static qbs *strfullz=NULL; if (!strfullz) strfullz=qbs_new(0,0);
            //find path
            qbs_set(strpath,strz);
            for(i=strpath->len;i>0;i--){
                if ((strpath->chr[i-1]==47)||(strpath->chr[i-1]==92)){strpath->len=i; break;}
            }//i
            if (i==0) strpath->len=0;//no path specified
            static int32 count;
            count=0;
            hFind = FindFirstFile(fixdir(strz), &fd);
            if(hFind==INVALID_HANDLE_VALUE){error(53); return;}//file not found
            do{
                if ((fd.dwFileAttributes&FILE_ATTRIBUTE_DIRECTORY)==0){
                    qbs_set(strfullz,qbs_add(strpath,qbs_new_txt_len(fd.cFileName,strlen(fd.cFileName)+1)));
                    if (!DeleteFile((char*)strfullz->chr)){
                        i=GetLastError();
                        if ((i==5)||(i==19)||(i==33)||(i==32)){FindClose(hFind); error(55); return;}//file already open
                        FindClose(hFind); error(53); return;//file not found
                    }
                    count++;
                }//not a directory
            }while(FindNextFile(hFind,&fd));
            FindClose(hFind);
            if (!count){error(53); return;}//file not found 
            return;
            #else
            if (remove(fixdir(strz))){
                i=errno;
                if (i==ENOENT){error(53); return;}//file not found
                if (i==EACCES){error(75); return;}//path/file access error
                error(64);//bad file name (assumed)
            }
        #endif
    }
    
    void sub_name(qbs *oldname,qbs *newname){
        if (new_error) return;
        static qbs *strz=NULL;
        if (!strz) strz=qbs_new(0,0);
        static qbs *strz2=NULL;
        if (!strz2) strz2=qbs_new(0,0);
        static int32 i;
        qbs_set(strz,qbs_add(oldname,qbs_new_txt_len("\0",1)));
        qbs_set(strz2,qbs_add(newname,qbs_new_txt_len("\0",1)));
        if (rename(fixdir(strz),fixdir(strz2))){
            i=errno;
            if (i==ENOENT){error(53); return;}//file not found
            if (i==EINVAL){error(64); return;}//bad file name
            if (i==EACCES){error(75); return;}//path/file access error
            error(5);//Illegal function call (assumed)
        }
    }
    
    void sub_chdir(qbs *str){
        
        
        
        if (new_error) return;
        static qbs *strz=NULL;
        if (!strz) strz=qbs_new(0,0);
        qbs_set(strz,qbs_add(str,qbs_new_txt_len("\0",1)));
        if (chdir(fixdir(strz))==-1){
            //assume errno==ENOENT
            error(76);//path not found
        }
        
        static int32 tmp_long;
        static int32 got_ports=0;
        
    }
    
    void sub_mkdir(qbs *str){
        if (new_error) return;
        static qbs *strz=NULL;
        if (!strz) strz=qbs_new(0,0);
        qbs_set(strz,qbs_add(str,qbs_new_txt_len("\0",1)));
        #ifdef QBNex_UNIX
            if (mkdir(fixdir(strz),0770)==-1){
                #else
                if (mkdir(fixdir(strz))==-1){
                #endif
                if (errno==EEXIST){error(75); return;}//path/file access error
                //assume errno==ENOENT
                error(76);//path not found
            }
            
        }
        
        void sub_rmdir(qbs *str){
            if (new_error) return;
            static qbs *strz=NULL;
            if (!strz) strz=qbs_new(0,0);
            qbs_set(strz,qbs_add(str,qbs_new_txt_len("\0",1)));
            if (rmdir(fixdir(strz))==-1){
                if (errno==ENOTEMPTY){error(75); return;}//path/file access error
                //assume errno==ENOENT
                error(76);//path not found
            }
        }
// ---- lines 22562,22653
        #ifdef QBNex_WINDOWS
            #define envp _environ
        #else
            extern char **environ;
            #define envp environ
        #endif

        int32 func__environcount() {
            //count array bound
             char **p = envp;
             while (*++p);
             return p - envp;
         }
             
        qbs *func_environ(qbs *name) {
            char *result;
            qbs *tqbs;
            std::string query((char*)name->chr, (char*)name->chr + name->len);
            result = getenv(query.c_str());
            if (result) {
                int result_length = strlen(result);
                tqbs = qbs_new(result_length, 1);
                memcpy(tqbs->chr, result, result_length);
            }
            else {
                tqbs = qbs_new(0, 1);
            }
            return tqbs;
        }

        qbs *func_environ(int32 number) {
            char *result;
            qbs *tqbs;
            int result_length;
            if (number <= 0) {
                tqbs = qbs_new(0, 1);
                error(5);
                return tqbs;
            }
            //Check we do not go beyond array bound
            char **p = envp;
            while (*++p);
            if (number > p - envp) {
                tqbs = qbs_new(0, 1);
                return tqbs;
            }
            result = envp[number - 1];
            result_length = strlen(result);
            tqbs = qbs_new(result_length, 1);
            memcpy(tqbs->chr, result, result_length);
            return tqbs;
        }
        
        void sub_environ(qbs *str) {
            char *separator;
            std::string buffer((char*)str->chr, (char*)str->chr + str->len);
            buffer.push_back('\0');
            char *buf=&buffer[0];
            //Name and value may be separated by = or space
            separator = strchr(buf, ' ');
            if (!separator) {
                separator = strchr(buf, '=');
            }
            if (!separator) {
                //It is an error is there is no separator
                error(5);
                return;
            }
            // Split into two separate strings
            *separator = '\0';
            if (separator == &buf[str->len] - 1) {
                //Separator is at end of string, so remove the variable
				#ifdef QBNex_WINDOWS
					*separator = '=';
					_putenv(buf);
				#else
					unsetenv(buf);
				#endif
            }
            else {
				#ifdef QBNex_WINDOWS
					#if WINVER >= 0x0600
					_putenv_s(buf, separator + 1);
					#else
					*separator = '=';
					_putenv(buf);
					#endif
				#else
					setenv(buf, separator + 1, 1);
				#endif
            }
        }
// ---- lines 27300,27345
        int32 func__fileexists(qbs* file){
            if (new_error) return 0;
            static qbs *strz=NULL;
            if (!strz) strz=qbs_new(0,0);
            qbs_set(strz,qbs_add(file,qbs_new_txt_len("\0",1)));
            #ifdef QBNex_WINDOWS
                static int32 x;
                x=GetFileAttributes(fixdir(strz));
                if (x==INVALID_FILE_ATTRIBUTES) return 0;
                if (x&FILE_ATTRIBUTE_DIRECTORY) return 0;
                return -1;
                #elif defined(QBNex_UNIX)
                struct stat sb;
                if (stat(fixdir(strz),&sb) == 0 && S_ISREG(sb.st_mode)) return -1;
                return 0;
                #else
                //generic method (not currently used)
                static ifstream fh;
                fh.open(fixdir(strz),ios::binary|ios::in);
                if (fh.is_open()==NULL){fh.clear(ios::goodbit); return 0;}
                fh.clear(ios::goodbit);
                fh.close();
                return -1;
            #endif
        }
        
        int32 func__direxists(qbs* file){
            if (new_error) return 0;
            static qbs *strz=NULL;
            if (!strz) strz=qbs_new(0,0);
            qbs_set(strz,qbs_add(file,qbs_new_txt_len("\0",1)));
            #ifdef QBNex_WINDOWS
                static int32 x;
                x=GetFileAttributes(fixdir(strz));
                if (x==INVALID_FILE_ATTRIBUTES) return 0;
                if (x&FILE_ATTRIBUTE_DIRECTORY) return -1;
                return 0;
                #elif defined(QBNex_UNIX)
                struct stat sb;
                if (stat(fixdir(strz),&sb) == 0 && S_ISDIR(sb.st_mode)) return -1;
                return 0;
                #else
                return 0;//default response
            #endif
        }
        
// ---- lines 28149,28190
qbs *func__cwd(){
    qbs *final=NULL, *tqbs=NULL;
    int length;
    std::vector<char> buf;
    
    #if defined QBNex_WINDOWS
        length = GetCurrentDirectoryA(0, NULL);
        buf.resize(length);
        if (buf.empty()) {
            error(7); //"Out of memory"
            return tqbs;
        }
        if (GetCurrentDirectoryA(length, buf.data()) != --length) { //Sanity check
            tqbs = qbs_new(0, 1);
            error(51); //"Internal error"
            return tqbs;
        }
        #elif defined QBNex_UNIX
        length = 512;
        while(1) {
            buf.resize(length);
            if (buf.empty()) {
                tqbs = qbs_new(0, 1);
                error(7);
                return tqbs;
            }
            char *ret = getcwd(buf.data(), length);
            if (ret) break;
            if (errno != ERANGE) {
                tqbs = qbs_new(0, 1);
                error(51);
                return tqbs;
            }
            length += 512;
        }
        length = strlen(buf.data());
    #endif
    final = qbs_new(length, 1);
    memcpy(final->chr, buf.data(), length);
    return final;
}

// ---- lines 25922,26029
        void sub_files(qbs *str,int32 passed){
            if (new_error) return;
            
            static int32 i,i2,i3;
            static qbs *strz=NULL; if (!strz) strz=qbs_new(0,0);
            
            if (passed){
                qbs_set(strz,qbs_add(str,qbs_new_txt_len("\0",1)));
                }else{
                qbs_set(strz,qbs_new_txt_len("\0",1));
            }
            
            
            
            #ifdef QBNex_WINDOWS
                static WIN32_FIND_DATA fd;
                static HANDLE hFind;
                static qbs *strpath=NULL; if (!strpath) strpath=qbs_new(0,0);
                static qbs *strz2=NULL; if (!strz2) strz2=qbs_new(0,0);
                
                i=0;
                if (strz->len>=2){
                    if (strz->chr[strz->len-2]==92) i=1;
                }else i=1;
                if (i){//add * (and new NULL term.)
                    strz->chr[strz->len-1]=42;//"*"
                    qbs_set(strz,qbs_add(strz,qbs_new_txt_len("\0",1))); 
                }
                
                qbs_set(strpath,strz);
                
                for(i=strpath->len;i>0;i--){
                    if ((strpath->chr[i-1]==47)||(strpath->chr[i-1]==92)){strpath->len=i; break;}
                }//i
                if (i==0) strpath->len=0;//no path specified
                
                //print the current path
                //note: for QBASIC compatibility reasons it does not print the directory name of the files being displayed
                static uint8 curdir[4096];
                static uint8 curdir2[4096];
                i2=GetCurrentDirectory(4096,(char*)curdir);
                if (i2){
                    i2=GetShortPathName((char*)curdir,(char*)curdir2,4096);
                    if (i2){
                        qbs_set(strz2,qbs_ucase(qbs_new_txt_len((char*)curdir2,i2)));
                        qbs_print(strz2,1);
                        }else{
                        error(5); return;
                    }
                    }else{
                    error(5); return;
                }
                
                hFind = FindFirstFile(fixdir(strz), &fd);
                if(hFind==INVALID_HANDLE_VALUE){error(53); return;}//file not found
                do{
                    
                    if (!fd.cAlternateFileName[0]){//no alternate filename exists
                        qbs_set(strz2,qbs_ucase(qbs_new_txt_len(fd.cFileName,strlen(fd.cFileName))));
                        }else{
                        qbs_set(strz2,qbs_ucase(qbs_new_txt_len(fd.cAlternateFileName,strlen(fd.cAlternateFileName))));
                    }
                    
                    if (strz2->len<12){//padding required
                        qbs_set(strz2,qbs_add(strz2,func_space(12-strz2->len)));
                        i2=0;
                        for (i=0;i<12;i++){
                            if (strz2->chr[i]==46){
                                memmove(&strz2->chr[8],&strz2->chr[i],4);
                                memset(&strz2->chr[i],32,8-i);
                                break;
                            }
                        }//i
                    }//padding
                    
                    //add "      " or "<DIR> "
                    if (fd.dwFileAttributes&FILE_ATTRIBUTE_DIRECTORY){
                        qbs_set(strz2,qbs_add(strz2,qbs_new_txt_len("<DIR> ",6)));
                        }else{
                        qbs_set(strz2,qbs_add(strz2,func_space(6)));
                    }
                    
                    makefit(strz2);
                    qbs_print(strz2,0);
                    
                }while(FindNextFile(hFind,&fd));
                FindClose(hFind);
                
                static ULARGE_INTEGER FreeBytesAvailableToCaller;
                static ULARGE_INTEGER TotalNumberOfBytes;
                static ULARGE_INTEGER TotalNumberOfFreeBytes;
                static int64 bytes;
                static char *cp;
                qbs_set(strpath,qbs_add(strpath,qbs_new_txt_len("\0",1)));
                cp=(char*)strpath->chr;
                if (strpath->len==1) cp=NULL;
                if (GetDiskFreeSpaceEx(cp,&FreeBytesAvailableToCaller,&TotalNumberOfBytes,&TotalNumberOfFreeBytes)){
                    bytes=*(int64*)(void*)&FreeBytesAvailableToCaller;
                    }else{
                    bytes=0;
                }
                if (func_pos(NULL)>1){strz2->len=0; qbs_print(strz2,1);}//new line if necessary
                qbs_set(strz2,qbs_add(qbs_str(bytes),qbs_new_txt_len(" Bytes free",11)));
                qbs_print(strz2,1);
                
            #endif
        }
        
