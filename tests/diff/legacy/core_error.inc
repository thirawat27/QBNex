// Verbatim legacy code from internal/c/libqb.cpp (commit ce58afd), used by tests/diff/diff_core.cpp.
// Do not edit: it is the reference the Rust port is compared against.

char *human_error(int32 errorcode) {
    switch (errorcode) {
        case 0: return "No error";
        case 1: return "NEXT without FOR";
        case 2: return "Syntax error";
        case 3: return "RETURN without GOSUB";
        case 4: return "Out of DATA";
        case 5: return "Illegal function call";
        case 6: return "Overflow";
        case 7: return "Out of memory";
        case 8: return "Label not defined";
        case 9: return "Subscript out of range";
        case 10: return "Duplicate definition";
        case 12: return "Illegal in direct mode";
        case 13: return "Type mismatch";
        case 14: return "Out of string space";
        //error 15 undefined
        case 16: return "String formula too complex";
        case 17: return "Cannot continue";
        case 18: return "Function not defined";
        case 19: return "No RESUME";
        case 20: return "RESUME without error";
        //error 21-23 undefined
        case 24: return "Device timeout";
        case 25: return "Device fault";
        case 26: return "FOR without NEXT";
        case 27: return "Out of paper";
        //error 28 undefined
        case 29: return "WHILE without WEND";
        case 30: return "WEND without WHILE";
        //error 31-32 undefined
        case 33: return "Duplicate label";
        //error 34 undefined
        case 35: return "Subprogram not defined";
        //error 36 undefined
        case 37: return "Argument-count mismatch";
        case 38: return "Array not defined";
        case 40: return "Variable required";
        case 50: return "FIELD overflow";
        case 51: return "Internal error";
        case 52: return "Bad file name or number";
        case 53: return "File not found";
        case 54: return "Bad file mode";
        case 55: return "File already open";
        case 56: return "FIELD statement active";
        case 57: return "Device I/O error";
        case 58: return "File already exists";
        case 59: return "Bad record length";
        case 61: return "Disk full";
        case 62: return "Input past end of file";
        case 63: return "Bad record number";
        case 64: return "Bad file name";
        case 67: return "Too many files";
        case 68: return "Device unavailable";
        case 69: return "Communication-buffer overflow";
        case 70: return "Permission denied";
        case 71: return "Disk not ready";
        case 72: return "Disk-media error";
        case 73: return "Feature unavailable";
        case 74: return "Rename across disks";
        case 75: return "Path/File access error";
        case 76: return "Path not found";
        case 258: return "Invalid handle";
        case 300: return "Memory region out of range";
        case 301: return "Invalid size";
        case 302: return "Source memory region out of range";
        case 303: return "Destination memory region out of range";
        case 304: return "Source and destination memory regions out of range";
        case 305: return "Source memory has been freed";
        case 306: return "Destination memory has been freed";
        case 307: return "Memory already freed";
        case 308: return "Memory has been freed";
        case 309: return "Memory not initialized";
        case 310: return "Source memory not initialized";
        case 311: return "Destination memory not initialized";
        case 312: return "Source and destination memory not initialized";
        case 313: return "Source and destination memory have been freed";
        case 314: return "_ASSERT failed";
        case 315: return "_ASSERT failed (check console for description)";
        default: return "Unprintable error";
    }
}

qbs *func_mid(qbs *str,int32 start,int32 l,int32 passed);
qbs *qbs_new_txt_len(const char *txt,int32 len);
qbs *func_command(int32 index, int32 passed);

void fix_error(){
    const char *cp;
    int prevent_handling = 0, len, v;
    if ((new_error >= 300) && (new_error <= 315)) prevent_handling = 1;
    if (!error_goto_line || error_handling || prevent_handling) {
        //strip path from binary name
        static int32 i;
        static qbs *binary_name=NULL;
        if (!binary_name) binary_name=qbs_new(0,0);
        qbs_set(binary_name,qbs_add(func_command( 0 ,1),qbs_new_txt_len("\0",1)));
        for(i=binary_name->len;i>0;i--){
            if ((binary_name->chr[i-1]==47)||(binary_name->chr[i-1]==92)) {
                qbs_set(binary_name,func_mid(binary_name, i + 1,NULL,0));
                break;
            }
        }
        
        cp = human_error(new_error);        
        #define FIXERRMSG_TITLE "%s%u - %s"
        #define FIXERRMSG_BODY "Line: %u (in %s)\n%s%s"
        #define FIXERRMSG_MAINFILE "main module"
        #define FIXERRMSG_CONT "\nContinue?"
        #define FIXERRMSG_UNHAND "Unhandled Error #"
        #define FIXERRMSG_CRIT "Critical Error #"
        
        len = snprintf(NULL, 0, FIXERRMSG_BODY, (inclercl ? inclercl : ercl), (inclercl ? includedfilename : FIXERRMSG_MAINFILE), cp, (!prevent_handling ? FIXERRMSG_CONT : ""));
        std::vector<char> errmess(static_cast<size_t>(len) + 1, '\0');
        snprintf(errmess.data(), errmess.size(), FIXERRMSG_BODY, (inclercl ? inclercl : ercl), (inclercl ? includedfilename : FIXERRMSG_MAINFILE), cp, (!prevent_handling ? FIXERRMSG_CONT : ""));
        
        len = snprintf(NULL, 0, FIXERRMSG_TITLE, (!prevent_handling ? FIXERRMSG_UNHAND : FIXERRMSG_CRIT), new_error, binary_name->chr);
        std::vector<char> errtitle(static_cast<size_t>(len) + 1, '\0');
        snprintf(errtitle.data(), errtitle.size(), FIXERRMSG_TITLE, (!prevent_handling ? FIXERRMSG_UNHAND : FIXERRMSG_CRIT), new_error, binary_name->chr);
        
        if (prevent_handling){
            v=MessageBox2(NULL,errmess.data(),errtitle.data(),MB_OK);
            exit(0);
            }else{
            v=MessageBox2(NULL,errmess.data(),errtitle.data(),MB_YESNO|MB_SYSTEMMODAL);
        }
        
        if ((v==IDNO)||(v==IDOK)){close_program=1; end();}
        new_error=0;
        return;
    }
    error_err=new_error;
    new_error=0;
    error_erl=last_line;
    error_occurred=1;
    QBMAIN (NULL);
    return;
}



void error(int32 error_number){
    
    //critical errors:
    
    //out of memory errors
    if (error_number==257){MessageBox2(NULL,"Out of memory","Critical Error #1",MB_OK|MB_SYSTEMMODAL); exit(0);}//generic "Out of memory" error
    //tracable "Out of memory" errors
    if (error_number==502){MessageBox2(NULL,"Out of memory","Critical Error #2",MB_OK|MB_SYSTEMMODAL); exit(0);}
    if (error_number==503){MessageBox2(NULL,"Out of memory","Critical Error #3",MB_OK|MB_SYSTEMMODAL); exit(0);}
    if (error_number==504){MessageBox2(NULL,"Out of memory","Critical Error #4",MB_OK|MB_SYSTEMMODAL); exit(0);}
    if (error_number==505){MessageBox2(NULL,"Out of memory","Critical Error #5",MB_OK|MB_SYSTEMMODAL); exit(0);}
    if (error_number==506){MessageBox2(NULL,"Out of memory","Critical Error #6",MB_OK|MB_SYSTEMMODAL); exit(0);}
    if (error_number==507){MessageBox2(NULL,"Out of memory","Critical Error #7",MB_OK|MB_SYSTEMMODAL); exit(0);}
    if (error_number==508){MessageBox2(NULL,"Out of memory","Critical Error #8",MB_OK|MB_SYSTEMMODAL); exit(0);}
    if (error_number==509){MessageBox2(NULL,"Out of memory","Critical Error #9",MB_OK|MB_SYSTEMMODAL); exit(0);}
    if (error_number==510){MessageBox2(NULL,"Out of memory","Critical Error #10",MB_OK|MB_SYSTEMMODAL); exit(0);}
    if (error_number==511){MessageBox2(NULL,"Out of memory","Critical Error #11",MB_OK|MB_SYSTEMMODAL); exit(0);}
    if (error_number==512){MessageBox2(NULL,"Out of memory","Critical Error #12",MB_OK|MB_SYSTEMMODAL); exit(0);}
    if (error_number==513){MessageBox2(NULL,"Out of memory","Critical Error #13",MB_OK|MB_SYSTEMMODAL); exit(0);}
    if (error_number==514){MessageBox2(NULL,"Out of memory","Critical Error #14",MB_OK|MB_SYSTEMMODAL); exit(0);}
    if (error_number==515){MessageBox2(NULL,"Out of memory","Critical Error #15",MB_OK|MB_SYSTEMMODAL); exit(0);}
    if (error_number==516){MessageBox2(NULL,"Out of memory","Critical Error #16",MB_OK|MB_SYSTEMMODAL); exit(0);}
    if (error_number==517){MessageBox2(NULL,"Out of memory","Critical Error #17",MB_OK|MB_SYSTEMMODAL); exit(0);}
    if (error_number==518){MessageBox2(NULL,"Out of memory","Critical Error #18",MB_OK|MB_SYSTEMMODAL); exit(0);}
    
    //other critical errors
    if (error_number==11){MessageBox2(NULL,"Division by zero","Critical Error",MB_OK|MB_SYSTEMMODAL); exit(0);}
    if (error_number==256){MessageBox2(NULL,"Out of stack space","Critical Error",MB_OK|MB_SYSTEMMODAL); exit(0);}
    if (error_number==259){MessageBox2(NULL,"Cannot find dynamic library file","Critical Error",MB_OK|MB_SYSTEMMODAL); exit(0);}
    if (error_number==260){MessageBox2(NULL,"Sub/Function does not exist in dynamic library","Critical Error",MB_OK|MB_SYSTEMMODAL); exit(0);}
    if (error_number==261){MessageBox2(NULL,"Sub/Function does not exist in dynamic library","Critical Error",MB_OK|MB_SYSTEMMODAL); exit(0);}
    
    if (error_number==270){MessageBox2(NULL,"_GL command called outside of SUB _GL's scope","Critical Error",MB_OK|MB_SYSTEMMODAL); exit(0);}
    if (error_number==271){MessageBox2(NULL,"END/SYSTEM called within SUB _GL's scope","Critical Error",MB_OK|MB_SYSTEMMODAL); exit(0);}
    
    
    if (!new_error){
        if ((new_error==256)||(new_error==257)) fix_error();//critical error!
        if (error_number<=0) error_number=5;//Illegal function call
        new_error=error_number;
        qbevent=1;
    }
}

