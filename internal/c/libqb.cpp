#include "common.h"
#include "libqb.h"
#include "../../crates/qb-runtime/include/qb_runtime.h" // Rust runtime (crates/qb-runtime)


#ifdef QBNex_WINDOWS
    #include <fcntl.h>
    #include <shellapi.h>
#endif

#ifdef QBNex_MACOSX
    #include <ApplicationServices/ApplicationServices.h>
    #include <CoreText/CoreText.h>
    
    #include "Cocoa/Cocoa.h"
    #include <CoreFoundation/CoreFoundation.h>
    #include <objc/objc.h>
    #include <objc/objc-runtime.h>
    
    #include <mach-o/dyld.h> //required for _NSGetExecutablePath
    
#endif

int32 disableEvents=0;

//Global console vvalues
static int32 consolekey;
static int32 consolemousex;
static int32 consolemousey;
static int32 consolebutton;
int32 func__getconsoleinput(); //declare here, so we can use with SLEEP and END commands

#ifdef QBNex_WINDOWS
static UINT qb_console_original_input_cp = 0;
static UINT qb_console_original_output_cp = 0;
static int32 qb_console_utf8_initialized = 0;

static int32 qb_is_console_handle(HANDLE handle) {
    DWORD mode = 0;
    if (handle == NULL || handle == INVALID_HANDLE_VALUE) return 0;
    return GetConsoleMode(handle, &mode) != 0;
}

static void qb_init_windows_console_utf8() {
    if (qb_console_utf8_initialized) return;
    qb_console_utf8_initialized = 1;

    qb_console_original_input_cp = GetConsoleCP();
    qb_console_original_output_cp = GetConsoleOutputCP();

    if (!qb_console_original_input_cp) qb_console_original_input_cp = GetACP();
    if (!qb_console_original_output_cp) qb_console_original_output_cp = GetACP();

    HANDLE stdout_handle = GetStdHandle(STD_OUTPUT_HANDLE);
    if (qb_is_console_handle(stdout_handle)) {
        SetConsoleOutputCP(CP_UTF8);
        DWORD output_mode = 0;
        if (GetConsoleMode(stdout_handle, &output_mode)) {
            SetConsoleMode(stdout_handle, output_mode | ENABLE_PROCESSED_OUTPUT);
        }
    }

    HANDLE stdin_handle = GetStdHandle(STD_INPUT_HANDLE);
    if (qb_is_console_handle(stdin_handle)) {
        SetConsoleCP(CP_UTF8);
    }
}

static int32 qb_multibyte_to_wide(const char* text, int32 len, UINT codepage, DWORD flags, std::wstring& out) {
    int required;

    out.clear();
    if (!text || len <= 0) return 1;

    required = MultiByteToWideChar(codepage, flags, text, len, NULL, 0);
    if (required <= 0) return 0;

    out.resize(required);
    return MultiByteToWideChar(codepage, flags, text, len, &out[0], required) > 0;
}
#endif

void qb_write_utf8(const char* text, int32 len, int32 finish_on_new_line) {
    if (!text) len = 0;
    if (len < 0 && text) len = static_cast<int32>(strlen(text));

    #ifdef QBNex_WINDOWS
    HANDLE stdout_handle = GetStdHandle(STD_OUTPUT_HANDLE);
    if (qb_is_console_handle(stdout_handle)) {
        std::wstring wide_text;
        DWORD written = 0;

        qb_init_windows_console_utf8();

        if (text && len > 0) {
            if (!qb_multibyte_to_wide(text, len, CP_UTF8, MB_ERR_INVALID_CHARS, wide_text)) {
                UINT fallback_codepage = qb_console_original_output_cp ? qb_console_original_output_cp : GetACP();
                if (!qb_multibyte_to_wide(text, len, fallback_codepage, 0, wide_text)) {
                    wide_text.clear();
                }
            }

            if (!wide_text.empty()) {
                WriteConsoleW(stdout_handle, wide_text.data(), static_cast<DWORD>(wide_text.size()), &written, NULL);
            }
        }

        if (finish_on_new_line) {
            static const wchar_t newline[] = L"\r\n";
            WriteConsoleW(stdout_handle, newline, 2, &written, NULL);
        }
        return;
    }
    #endif

    if (text && len > 0) {
        fwrite(text, 1, len, stdout);
    }
    if (finish_on_new_line) {
        fwrite("\n", 1, 1, stdout);
    }
    fflush(stdout);
}

//This next block used to be in common.cpp; put here until I can find a better
//place for it (LC, 2018-01-05)

uint32 rotateLeft(uint32 word,uint32 shift){
return (word << shift) | (word >> (32 - shift));
}
    
#ifndef QBNex_WINDOWS
    void Sleep(uint32 milliseconds){
        uint64 sec  = milliseconds / 1000;
        uint64 nsec = (milliseconds % 1000) * 1000000ULL;
        timespec ts;
        ts.tv_sec  = static_cast<time_t>(sec);
        ts.tv_nsec = static_cast<long>(nsec);
        nanosleep(&ts, NULL);
    }
    
    void ZeroMemory(void *ptr,int64 bytes){
        memset(ptr,0,bytes);
    }
#endif
// QBasic-compatible rounding lives in the Rust runtime (crates/qb-runtime/src/round.rs).
#include <float.h>
#if LDBL_MANT_DIG == 64
    int64 qbr(long double f){return qb_qbr_f80((const unsigned char*)&f);}
    uint64 qbr_longdouble_to_uint64(long double f){return (uint64)qb_fistp_i64_f80((const unsigned char*)&f);}
#else
    int64 qbr(long double f){return qb_qbr_f64((double)f);}
    uint64 qbr_longdouble_to_uint64(long double f){return (uint64)qb_fistp_i64_f64((double)f);}
#endif
int32 qbr_float_to_long(float f){return qb_fistp_i32(f);}
int32 qbr_double_to_long(double f){return qb_fistp_i32(f);}
void fpu_reinit(){qb_fpu_reinit();}
//bit-array access functions (note: used to be included through 'bit.cpp')
uint64 getubits(uint32 bsize,uint8 *base,ptrszint i){return qb_getubits(bsize,base,i);}
int64 getbits(uint32 bsize,uint8 *base,ptrszint i){return qb_getbits(bsize,base,i);}
void setbits(uint32 bsize,uint8 *base,ptrszint i,int64 val){qb_setbits(bsize,base,i,val);}

#ifdef QBNex_UNIX
    #include <pthread.h>
    #include <libgen.h> //required for dirname()
    #include <sys/ioctl.h> //required for terminal width and height
#endif

#ifdef QBNex_LINUX
    #include <X11/Xlib.h>
    #include <X11/Xutil.h>
    #include <X11/Xatom.h>
    Display *X11_display=NULL;
    Window X11_window;
#endif

static std::mutex x11_lock_mutex;
static std::condition_variable x11_lock_cv;
static bool x11_locked=false;
static bool x11_lock_request=false;
void x11_lock(){
    std::unique_lock<std::mutex> lock(x11_lock_mutex);
    x11_lock_request=true;
    x11_lock_cv.notify_all();
    x11_lock_cv.wait(lock, [](){ return x11_locked; });
}
void x11_unlock(){
    std::lock_guard<std::mutex> lock(x11_lock_mutex);
    x11_locked=false;
    x11_lock_cv.notify_all();
}



/*
    Logging for QBNex developers (when an alert() just isn't enough)
    1) Temporarily set allow_logging=1
    2) Call log with a string or number:
    log_event("this is a char* string");
    log_event(12345);
    3) 'log.txt' is created in the same folder as your executable
    * 'log.txt' is truncated every time your program runs on the first call to log_event(...)
*/
int32 allow_logging=1;
std::ofstream log_file;
int32 log_file_opened=0;
void open_log_file(){
    if (log_file_opened==0){
        log_file.open("log.txt", std::ios_base::out|std::ios_base::trunc);
        log_file_opened=1;    
    }
}
void log_event(char *x){
    open_log_file();
    log_file << x;  
}
void log_event(int32 x){
    open_log_file();
    char str[1000];
    memset(&str[0],0,1000);
    sprintf(str, "%d", x);
    log_file << &str[0];
}


void sub__printimage(int32 i);//crates/qb-runtime/src/printer.rs via the shim near sub_files

void alert(int32 x);
void alert(char *x);

//GUI notification variables
int32 force_display_update=0;

void sub__delay(double seconds);

void *generic_window_handle=NULL;
int32 acceptFileDrop=0;
#ifdef QBNex_WINDOWS
    HWND window_handle=NULL;
#endif
//...

extern "C" void QBNex_Window_Handle(void *handle){
    generic_window_handle=handle;
    #ifdef QBNex_WINDOWS
        window_handle=(HWND)handle; 
    #endif
    //...
}



//forward references
void set_view(int32 new_mode);
void set_render_source(int32 new_handle);
void set_render_dest(int32 new_handle);

int32 framebufferobjects_supported=0;

int32 environment_2d__screen_width=0; //the size of the software SCREEN
int32 environment_2d__screen_height=0;
int32 environment__window_width=0; //window may be larger or smaller than the SCREEN
int32 environment__window_height=0;
int32 environment_2d__screen_x1=0; //offsets of 'screen' within the window
int32 environment_2d__screen_y1=0;
int32 environment_2d__screen_x2=0;
int32 environment_2d__screen_y2=0;
int32 environment_2d__screen_scaled_width=640;//inital values prevent _SCALEDWIDTH/_SCALEDHEIGHT returning 0
int32 environment_2d__screen_scaled_height=400;
float environment_2d__screen_x_scale=1.0f;
float environment_2d__screen_y_scale=1.0f;
int32 environment_2d__screen_smooth=0;//1(LINEAR) or 0(NEAREST)
int32 environment_2d__letterbox=0;//1=vertical black stripes required, 2=horizontal black stripes required




int32 window_exists=0;
std::vector<std::string> dropped_files;//R8: _DROPPEDFILE$ list (was a Win32 HDROP)
int32 create_window=0;
int32 window_focused=0; //Not used on Windows
uint8 *window_title=NULL;
static std::vector<uint8> window_title_storage;
int32 temp_window_title_set=0;

double max_fps=60;//60 is the default
int32 auto_fps=0;//set to 1 to make QBNex auto-adjust fps based on load

int32 os_resize_event=0;

int32 resize_auto=0;//1=_STRETCH, 2=_SMOOTH
float resize_auto_ideal_aspect=640.0/400.0;
float resize_auto_accept_aspect=640.0/400.0;

int32 fullscreen_allowedmode=0;
int32 fullscreen_allowedsmooth=0;
int32 fullscreen_smooth=0;
int32 fullscreen_width=0;
int32 fullscreen_height=0;
int32 screen_scale=0;
int32 resize_pending=1;
int32 resize_snapback=1;
int32 resize_snapback_x=640;
int32 resize_snapback_y=400;
int32 resize_event=0;
int32 resize_event_x=0;
int32 resize_event_y=0;

int32 ScreenResizeScale=0;
int32 ScreenResize=0;
extern "C" int QBNex_Resizable(){
    return ScreenResize;
}


int32 sub_gl_called=0;

extern void evnt(uint32 linenumber, uint32 inclinenumber = 0, const char* incfilename = NULL);

#ifdef QBNex_WINDOWS
    //R8: the freeglut OS event hooks (qbnex_os_event_*) are gone
#endif

#if defined(QBNex_LINUX) && defined(QBNex_GUI)

#endif

#define QBNex_EVENT_CLOSE 1
#define QBNex_EVENT_KEY 2
#define QBNex_EVENT_RELATIVE_MOUSE_MOVEMENT 3
#define QBNex_EVENT_FILE_DROP 4
#define QBNex_EVENT_KEY_PAUSE 1000

static int32 image_qbicon16_handle;
static int32 image_qbicon32_handle;

static int32 image_qbicon16_w= 16;
static int32 image_qbicon16_h= 16;
static const uint8 image_qbicon16[]={
    0,0,0,0,0,0,0,0,255,255,255,1,0,1,0,0,0,1,0,0,1,2,0,0,1,1,0,0,0,1,0,0,0,1,0,0,1,2,0,0,127,127,127,2,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,12,0,121,21,23,0,139,22,0,0,0,0,0,0,0,6,23,0,139,22,34,0,139,22,0,0,92,11,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,255,255,127,2,0,0,63,4,105,130,120,154,143,204,124,242,138,205,117,243,96,122,110,185,104,126,121,191,141,205,120,243,134,204,114,241,120,169,118,226,58,50,104,61,1,1,0,0,127,127,127,2,0,0,0,0,0,0,0,0,0,0,0,0,0,1,0,0,30,0,132,25,140,211,113,255,93,141,97,255,93,132,102,255,114,184,97,255,117,158,121,255,116,192,92,255,74,88,101,255,134,225,101,255,58,74,93,201,0,0,0,0,127,255,127,2,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,51,0,61,25,121,183,177,243,79,98,133,254,141,148,174,253,91,149,154,253,103,135,147,253,112,184,193,254,109,156,193,251,113,180,171,253,53,58,93,196,0,0,0,0,255,255,255,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,75,28,47,27,101,145,191,249,87,110,150,253,112,158,195,252,76,119,171,253,94,118,159,253,82,123,173,253,63,69,108,249,88,137,190,253,50,59,104,193,0,0,0,0,255,255,255,1,0,0,0,0,0,0,0,0,0,0,0,0,255,255,255,1,105,75,105,17,69,79,116,235,68,84,127,255,58,77,123,255,56,71,114,255,97,105,140,255,50,67,113,255,71,82,125,255,57,73,119,255,44,47,90,213,0,0,0,0,255,255,255,1,0,0,0,0,0,0,0,0,0,0,0,0,127,0,127,2,0,0,0,0,19,0,76,53,33,33,88,192,36,43,86,195,34,32,88,199,52,55,97,196,40,46,89,207,33,42,85,205,44,49,94,206,28,33,75,108,85,85,85,3,42,85,85,6,0,0,0,1,0,0,0,0,0,0,0,0,1,2,0,0,0,0,85,3,120,154,128,173,101,131,114,176,50,40,97,81,128,168,135,200,39,44,84,97,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,3,0,0,0,0,127,6,140,205,117,237,139,245,92,255,92,98,116,235,150,237,115,255,37,41,85,189,79,100,100,71,88,119,107,81,44,27,88,46,75,88,104,78,34,21,90,59,79,96,105,77,36,36,72,21,0,0,0,0,0,0,0,1,1,3,0,0,0,0,42,6,129,182,140,222,115,206,112,252,125,177,149,255,121,187,137,253,85,107,112,249,128,195,112,255,124,195,109,255,98,125,118,250,120,172,114,255,112,159,111,255,119,176,119,255,29,19,87,131,0,0,0,0,51,51,102,5,1,2,2,0,0,0,0,6,116,160,199,222,94,121,171,255,103,161,204,254,87,149,204,254,113,144,164,254,104,159,190,251,102,144,170,251,73,98,122,253,89,112,137,247,117,206,201,251,71,89,121,240,11,0,55,46,1,1,0,0,0,0,0,2,0,0,0,0,85,42,42,6,87,105,142,232,51,63,107,255,68,76,118,244,48,75,122,255,88,98,140,255,61,82,130,255,61,72,114,255,86,90,129,255,78,98,141,255,70,91,143,255,61,75,123,254,55,51,89,74,0,0,0,0,85,85,85,3,0,0,0,1,85,85,85,3,60,63,103,197,39,45,89,234,35,32,78,101,49,55,98,241,38,40,84,191,47,51,94,214,40,50,96,242,51,56,98,220,52,57,98,236,43,43,85,201,49,55,99,240,29,33,80,123,0,0,0,0,51,51,102,5,0,0,0,1,0,0,0,0,0,0,0,8,0,0,38,20,0,0,0,0,0,0,26,19,0,0,0,3,0,0,0,8,0,0,48,21,0,0,0,7,0,0,15,16,0,0,0,2,0,0,14,18,0,0,0,5,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,1,0,0,0,0,0,0,127,127,127,2,1,1,0,0,1,1,0,0,1,1,0,0,0,0,0,0,1,1,0,0,1,1,0,0,1,1,1,0,1,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0
};

static int32 image_qbicon32_w= 32;
static int32 image_qbicon32_h= 32;
static const uint8 image_qbicon32[]={
    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,0,0,0,1,0,0,0,1,0,0,0,2,0,0,0,1,0,0,0,0,0,0,0,0,0,0,0,1,0,0,0,2,0,0,0,1,0,0,0,2,0,0,0,1,0,0,0,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,255,255,255,1,3,4,0,0,2,4,0,0,3,4,0,0,2,4,0,0,2,3,1,0,0,0,127,2,0,0,0,0,3,3,0,0,2,4,0,0,2,4,0,0,3,4,0,0,2,4,0,0,3,4,0,0,255,255,255,1,0,0,0,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,1,1,1,0,0,0,0,0,0,0,0,26,0,0,0,26,0,0,0,26,0,0,0,28,0,0,0,12,0,0,0,0,0,0,0,2,0,0,0,26,0,0,0,27,0,0,0,26,0,0,0,27,0,0,0,26,0,0,0,26,0,0,0,0,1,1,1,0,0,0,0,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,1,1,1,0,0,0,0,10,62,53,108,176,136,144,154,245,133,152,145,244,133,150,146,245,134,154,148,244,102,104,133,227,0,0,45,61,26,26,76,87,125,137,148,245,134,154,146,242,133,151,146,245,133,151,146,245,133,152,146,244,132,146,149,244,64,57,107,182,0,0,0,14,1,1,1,0,0,0,0,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,127,255,127,2,51,51,0,5,64,58,102,179,177,224,163,255,161,255,102,255,148,255,92,255,150,255,96,255,153,255,94,255,157,238,125,255,88,89,120,240,51,40,100,203,181,255,151,255,147,255,89,255,150,255,96,255,149,255,94,255,149,255,94,255,155,255,97,255,142,187,138,255,63,55,105,196,0,0,0,10,127,127,127,2,0,0,0,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,2,2,3,0,0,0,0,22,34,133,150,147,255,156,255,95,252,94,166,82,253,84,103,108,254,76,97,101,254,95,145,97,254,134,248,76,253,101,142,107,255,67,52,120,255,159,236,129,254,113,227,72,254,78,103,101,254,82,108,105,254,75,95,104,254,108,193,84,254,129,243,81,249,84,89,125,255,14,10,66,127,0,0,0,0,0,0,63,4,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,2,2,3,0,0,0,0,21,35,136,160,144,250,149,255,87,255,94,128,104,254,47,41,103,255,27,25,83,255,83,96,112,255,151,255,89,255,88,137,90,254,62,49,116,253,166,243,134,255,115,224,72,255,78,71,115,255,28,22,80,255,31,13,88,255,135,193,120,255,130,253,83,251,69,80,112,253,16,12,71,158,0,0,0,0,0,0,63,4,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,2,2,3,0,0,0,0,14,35,133,153,152,251,134,238,158,255,85,121,118,254,57,55,101,255,231,230,237,255,121,137,144,255,128,230,154,255,83,125,124,255,66,56,107,255,158,223,171,255,103,202,138,255,126,168,176,255,154,192,199,255,155,192,195,255,144,232,154,255,98,171,112,251,79,82,120,255,11,12,66,157,0,0,0,0,0,0,63,4,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,2,2,3,1,0,0,0,0,35,129,146,171,252,113,205,243,255,78,109,148,254,55,53,93,255,160,152,170,255,106,115,145,255,109,199,237,255,75,112,154,255,69,61,97,255,140,195,221,255,96,184,226,255,91,168,206,255,95,172,209,255,93,172,206,255,89,178,214,255,99,125,164,251,80,80,118,255,0,1,56,131,1,1,0,0,0,0,63,4,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,2,2,3,1,0,0,0,0,35,128,144,173,252,116,207,255,255,73,102,147,254,82,88,122,255,102,160,203,255,140,175,204,255,110,195,244,255,76,113,159,255,69,60,95,255,143,198,233,255,85,164,219,255,72,74,116,255,63,67,109,255,60,59,99,255,92,144,192,255,101,185,235,251,78,82,123,255,15,11,64,131,0,0,0,0,0,0,63,4,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,2,1,2,0,0,0,0,7,35,122,133,161,250,90,159,204,255,68,89,132,254,106,104,131,255,112,156,193,255,75,136,182,255,81,147,193,255,68,94,138,255,72,66,102,255,125,165,196,255,65,125,174,255,83,84,119,255,29,32,80,255,37,32,79,255,113,146,177,255,62,117,166,251,65,70,113,255,16,19,68,159,0,0,0,0,0,0,63,4,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,2,1,1,0,0,0,0,20,37,117,119,149,255,37,60,110,253,32,50,99,254,124,132,159,255,119,125,151,255,30,53,105,255,19,44,97,255,90,97,132,255,82,82,117,255,96,111,147,255,12,36,91,255,89,100,135,255,119,129,159,255,123,132,161,255,65,85,126,255,13,34,89,251,69,72,114,254,18,20,69,151,0,0,0,0,0,0,63,4,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,127,127,127,2,0,0,28,9,64,61,100,182,94,98,135,255,28,46,98,249,20,39,93,251,20,40,94,251,34,47,96,251,37,51,100,251,38,52,101,251,121,120,147,251,101,111,145,251,19,36,90,251,26,45,97,251,22,41,94,251,22,41,94,251,18,37,91,251,62,70,111,247,82,88,124,255,6,7,59,163,1,0,0,0,0,0,63,4,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,1,1,1,0,0,0,0,10,65,62,104,188,81,84,125,255,66,72,113,255,66,71,112,255,74,80,119,255,86,90,126,255,62,68,111,255,74,80,120,255,85,89,126,255,63,68,110,255,68,73,113,255,67,73,113,255,67,73,113,255,65,70,111,255,77,83,120,255,19,26,74,255,12,9,60,84,0,0,0,0,0,0,85,3,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,2,0,0,0,0,0,0,0,0,0,0,43,134,8,13,63,149,10,15,64,163,5,8,59,150,0,0,42,143,0,0,48,146,0,1,55,147,3,4,59,159,10,15,64,163,11,14,62,162,11,15,62,162,10,15,62,163,11,15,63,160,3,7,61,163,11,8,60,88,0,0,0,0,0,0,0,2,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,127,2,0,0,0,0,34,31,77,105,86,79,126,163,96,86,134,148,33,30,78,85,0,0,0,0,36,36,82,77,91,84,129,148,97,90,135,149,45,42,85,95,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,2,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,3,4,2,0,0,0,0,0,102,103,132,220,193,255,161,255,158,235,131,255,97,99,134,255,3,3,43,70,94,91,130,192,197,255,167,255,150,214,132,255,67,64,116,255,0,3,47,70,95,95,95,8,46,46,92,11,72,72,109,7,72,72,109,7,42,42,85,6,42,42,85,6,72,72,109,7,127,127,127,4,85,0,85,3,85,85,85,3,127,127,127,4,127,127,127,4,0,0,85,3,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,3,4,2,0,0,0,0,0,113,117,141,204,161,255,113,251,123,253,64,247,117,170,115,254,48,38,98,210,85,85,122,196,171,255,120,251,104,197,73,245,57,55,111,253,3,9,57,84,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,3,4,2,0,0,0,0,0,112,114,139,207,164,254,121,255,138,255,81,253,130,234,84,255,85,95,116,255,87,81,127,254,169,254,120,255,110,196,84,253,60,59,112,254,0,0,40,106,51,45,98,134,70,71,111,156,69,69,106,151,72,69,115,154,16,9,72,77,7,11,58,65,66,66,109,154,67,64,110,154,25,25,73,101,26,26,75,105,69,69,110,154,65,63,106,153,24,20,68,74,0,0,0,0,0,0,127,2,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,3,4,2,0,0,0,0,0,112,113,139,207,166,255,118,255,116,218,77,253,130,240,84,255,100,170,84,254,137,127,163,255,165,255,116,255,110,197,83,255,52,49,103,254,56,56,97,244,162,232,134,255,147,251,104,255,148,248,107,255,156,249,118,255,96,107,118,252,60,48,108,233,181,255,149,255,132,213,105,255,67,56,117,255,105,125,125,255,167,255,114,255,111,152,120,255,36,30,91,232,0,0,0,12,255,255,0,1,0,0,0,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,3,4,2,0,0,0,0,0,110,112,137,207,156,246,180,255,79,137,122,253,94,129,126,255,106,212,155,255,135,151,170,255,152,231,177,255,94,168,141,255,67,57,115,255,166,215,157,255,126,235,81,251,85,110,89,251,81,94,95,251,114,203,75,251,119,195,99,255,94,82,137,255,128,167,128,252,116,239,73,251,103,125,116,254,163,228,140,253,110,220,73,250,95,109,121,251,34,36,88,246,15,0,15,17,1,1,0,0,0,0,0,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,3,3,3,0,0,0,0,0,112,112,141,207,143,218,255,255,80,134,184,253,125,119,149,255,91,161,217,255,102,177,227,255,109,187,238,255,88,149,202,255,75,69,110,255,150,200,197,254,98,181,182,255,116,150,201,255,120,154,197,255,127,202,195,255,98,178,166,254,51,52,99,253,65,55,108,249,116,162,144,255,114,206,170,255,110,197,162,255,90,110,125,252,60,60,111,255,6,9,60,164,63,63,63,4,0,0,127,2,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,3,3,3,0,0,0,0,0,109,109,140,207,137,209,247,255,72,124,174,253,84,78,109,255,104,130,165,253,95,177,228,255,104,186,234,255,83,140,188,255,75,69,104,255,144,188,224,255,94,171,227,255,77,112,156,255,80,120,161,255,77,112,161,255,84,105,153,255,55,61,101,254,38,33,79,241,154,194,227,255,89,171,226,255,90,166,222,253,100,122,162,255,36,35,79,224,0,0,0,1,1,1,0,0,0,0,0,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,3,3,3,0,0,0,0,0,110,109,140,207,89,124,164,255,48,73,119,253,48,50,95,255,88,86,125,255,72,99,142,254,44,86,134,255,48,75,123,255,83,81,116,255,126,156,186,253,48,91,140,255,84,79,112,255,69,70,104,255,101,104,130,255,90,97,127,255,64,66,105,255,106,121,154,255,70,121,165,255,67,85,129,255,75,106,147,254,51,91,139,252,95,92,129,249,37,34,74,82,1,1,0,0,0,0,85,3,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,3,3,2,0,0,0,0,0,113,112,142,206,63,83,126,254,23,37,88,253,55,57,99,249,27,26,77,184,91,91,126,255,14,35,88,253,27,44,94,254,73,76,113,254,102,104,137,255,23,41,91,253,36,57,106,254,53,71,115,254,30,53,102,254,39,51,100,254,123,125,149,252,84,103,139,253,6,22,77,254,88,90,121,253,88,91,123,253,11,31,86,251,55,64,110,255,49,49,93,215,0,0,0,7,255,255,255,1,0,0,0,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,3,3,2,0,0,0,0,0,94,92,127,214,90,98,135,255,65,72,111,254,62,67,109,255,0,0,21,71,78,77,114,202,80,87,127,255,66,74,114,255,58,63,105,255,40,38,81,226,94,97,131,255,53,63,106,255,56,65,108,255,60,68,110,255,82,87,120,255,62,69,111,255,101,106,139,255,74,81,118,255,53,59,102,255,63,63,107,255,76,83,121,255,76,82,120,255,38,43,87,247,0,0,14,18,1,1,0,0,0,0,0,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,85,3,0,0,0,0,17,14,66,104,29,33,82,241,31,38,83,240,18,24,75,233,17,17,68,30,0,0,49,46,30,35,84,233,30,38,84,241,20,24,75,236,0,0,36,56,25,28,76,173,33,39,87,244,31,37,82,241,33,38,83,243,15,23,73,241,6,5,61,150,28,31,79,225,32,39,86,244,14,17,67,199,10,7,60,102,31,35,83,238,30,37,83,244,15,19,69,198,0,0,63,8,0,0,0,1,0,0,0,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,34,22,0,0,30,25,0,0,30,17,0,0,0,0,0,0,0,0,0,0,15,17,0,0,40,25,0,0,40,19,1,1,0,0,0,0,0,1,0,0,31,24,0,0,33,23,0,0,31,24,11,0,57,22,0,0,0,0,0,0,17,15,0,0,40,25,0,0,42,6,0,0,0,0,0,0,25,20,0,0,40,25,0,0,31,8,0,0,0,0,0,0,0,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,255,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,0,0,0,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,2,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,85,3,0,0,0,0,0,0,0,0,0,0,0,0,0,0,85,3,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,0,0,0,1,0,0,0,1,0,0,0,0,0,0,0,0,0,0,0,1,0,0,0,1,0,0,0,1,0,0,0,0,0,0,0,1,0,0,0,1,0,0,0,1,0,0,0,1,0,0,0,1,0,0,0,0,0,0,0,1,0,0,0,1,0,0,0,1,0,0,0,0,0,0,0,1,0,0,0,1,0,0,0,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
};





static int32 display_x=640;
static int32 display_y=400;
int32 display_x_prev=640,display_y_prev=400;

static int32 display_required_x=640;
static int32 display_required_y=400;

int32 dont_call_sub_gl=0;



struct display_frame_struct{
    int32 state;
    int64 order;
    uint32 *bgra;
    std::vector<uint32> bgra_storage;
    int32 w;
    int32 h;
    int32 bytes;//w*h*4
};
display_frame_struct display_frame[3];
int64 display_frame_order_next=1;
#define DISPLAY_FRAME_STATE__EMPTY 1
#define DISPLAY_FRAME_STATE__BUILDING 2
#define DISPLAY_FRAME_STATE__READY 3
#define DISPLAY_FRAME_STATE__DISPLAYING 4

//when a new software frame is not required by display(), hardware content may exist,
//and if it does then this variable is used to determine to highest order index to render
int64 last_rendered_hardware_display_frame_order=0;
int64 last_hardware_display_frame_order=0;

struct MUTEX {
    std::mutex handle;
};

MUTEX* new_mutex(){
    return new MUTEX();
}

void free_mutex(MUTEX *mutex){
    delete mutex;
}

void lock_mutex(MUTEX *m){
    if (m==NULL) return;
    m->handle.lock();
}

void unlock_mutex(MUTEX *m){
    if (m==NULL) return;
    m->handle.unlock();
}

static std::unique_lock<std::mutex> make_unique_lock(MUTEX *mutex){
    if (mutex==NULL) return std::unique_lock<std::mutex>();
    return std::unique_lock<std::mutex>(mutex->handle);
}


//List Interface
//Purpose: Unify and optimize the way QBNex references lists of objects (such as handles)
//Notes: Does not use index 0
struct list{
    ptrszint user_structure_size=0;
    ptrszint internal_structure_size=0;
    uint8 *structure=NULL;//block of structures of user-specified size
    ptrszint structures=0;
    ptrszint structures_last=0;
    std::vector<ptrszint> structure_freed;//quickly re-reference available structures after they have been removed
    std::vector<ptrszint> structure_bases;//retain previous blocks so references stay valid
    std::vector<ptrszint> index;//pointers to the structures referred to by each index value
    ptrszint indexes=0;
    std::unique_ptr<MUTEX> lock;
};

//fwd refs
void *list_get(list *L, ptrszint i);



list *list_new(ptrszint structure_size){
    list *L=new list();
    L->user_structure_size=structure_size;
    L->internal_structure_size=structure_size+sizeof(ptrszint);
    L->index.push_back(0);
    return L;
}

list *list_new_threadsafe(ptrszint structure_size){
    list *L=list_new(structure_size);
    L->lock=std::unique_ptr<MUTEX>(new_mutex());
    return L;
}

ptrszint list_add(list *L){  
    std::unique_lock<std::mutex> list_lock=make_unique_lock(L->lock.get());
    ptrszint i;
    if(!L->structure_freed.empty()){//retrieve index from freed list if possible
        i=L->structure_freed.back();
        L->structure_freed.pop_back();
        uint8* structure;
        structure=(uint8*)L->index[i];
        memset(structure,0,L->user_structure_size);
        *(ptrszint*)(structure+L->user_structure_size)=i;
        }else{
        //create new buffer?
        if ((L->structures+1)>L->structures_last){
            ptrszint new_structures_last;
            new_structures_last=(L->structures_last*2)+1;
            //note: L->structure is only modified by list_add
            L->structure=(uint8*)calloc(1,L->internal_structure_size*(new_structures_last+1));
            if (L->structure==NULL){ alert("list_add: failed to allocate new buffer, structure size:"); alert(L->internal_structure_size);}
            L->structures_last=new_structures_last;
            L->structures=0;
            L->structure_bases.push_back((ptrszint)L->structure);
        }
        i=++L->indexes;    
        *(ptrszint*)(L->structure+(L->internal_structure_size*(++L->structures))+L->user_structure_size)=i;      
        //allocate new index
        if ((ptrszint)L->index.size()<=i){
            L->index.resize(i+1,0);
        }
        L->index[i]=(ptrszint)( L->structure + (L->internal_structure_size*L->structures) );
    }
    return i;
}//list_add

ptrszint list_remove(list *L,ptrszint i){//returns -1 on success, 0 on failure
    std::unique_lock<std::mutex> list_lock=make_unique_lock(L->lock.get());
    if ((i<1)||(i>L->indexes)){
        return 0;
    }
    uint8* structure;
    structure=(uint8*)(L->index[i]);
    if (!*(ptrszint*)(structure+L->user_structure_size)){
        return 0;
    }  
    L->structure_freed.push_back(i);
    *(ptrszint*)(structure+L->user_structure_size)=0;
    return -1;
};

void list_destroy(list *L){
    for (std::vector<ptrszint>::const_iterator it=L->structure_bases.begin(); it!=L->structure_bases.end(); ++it){
        free((void*)(*it));
    }
    delete L;
}

void *list_get(list *L, ptrszint i){//Returns a pointer to an index's structure
    std::unique_lock<std::mutex> list_lock=make_unique_lock(L->lock.get());
    if ((i<1)||(i>L->indexes)){
        return NULL;
    }
    uint8* structure;
    structure=(uint8*)(L->index[i]);
    if (!*(ptrszint*)(structure+L->user_structure_size)) return NULL;
    return (void*)structure;
}

ptrszint list_get_index(list *L,void *structure){//Retrieves the index value of a structure
    ptrszint i=*(ptrszint*) ( ((uint8*)structure) + L->user_structure_size );
    return i;
}

//Special Handle system
//---------------------
//Purpose: Manage handles to custom QBNex interfaces alongside the standard QB file handle indexing
//Method:  Uses negative values for custom interface handles
struct special_handle_struct{
    int8 type;
    //0=Invalid handle (was previously being used but was closed)
    //1=Stream (eg. COM, Network[TCP/IP])
    //2=Host Listener (used by _OPENCONNECTION)
    ptrszint index;//an index or pointer to the type's object
};
list *special_handles=NULL;

//Stream system
//-------------
//Purpose: Unify access to the input and/or output of streamed data
struct stream_struct{
    uint8 *in;
    ptrszint in_size;//current size in bytes
    ptrszint in_limit;//size before reallocation of buffer is required
    int8 eof;//user attempted to read past end of stream
    //Note: 'out' is unrequired because data can be sent directly to the interface
    //-----------------------------------------
    int8 type;
    //1=Network (TCP)
    ptrszint index;//an index or pointer to the type's object
};
list *stream_handles=NULL;
static std::unordered_map<stream_struct*, std::vector<uint8>> stream_input_buffers;

static void resize_stream_input_buffer(stream_struct *stream, ptrszint bytes){
    auto &buffer=stream_input_buffers[stream];
    buffer.resize(static_cast<size_t>(bytes));
    stream->in=buffer.empty()?NULL:buffer.data();
    stream->in_limit=bytes;
    if (stream->in_size>stream->in_limit) stream->in_size=stream->in_limit;
}

void stream_free(stream_struct *st){
    stream_input_buffers.erase(st);
    st->in=NULL;
    st->in_size=0;
    st->in_limit=0;
    list_remove(stream_handles,list_get_index(stream_handles,st));
}

void stream_update(stream_struct *stream);
void stream_out(stream_struct *st,void *offset,ptrszint bytes);

void connection_close(ptrszint i);


/********** Render State **********/
/*
    Apart from 'glTexParameter' based settings (with are texture specific)
    all other OpenGL states are global.
    This means when switching between dest FBOs a complete state change is inevitable.
*/
struct RENDER_STATE_DEST{ //could be the primary render target or a FBO
    int32 ignore;//at present no relevant states appear to be FBO specific
};
struct RENDER_STATE_SOURCE{ //texture states
    int32 smooth_stretched;
    int32 smooth_shrunk;
    int32 texture_wrap;
    int32 PO2_fix;
};
struct RENDER_STATE_GLOBAL{ //settings not bound to specific source/target
    RENDER_STATE_DEST *dest;
    RENDER_STATE_SOURCE *source;
    int32 dest_handle;        
    int32 source_handle;
    int32 view_mode;
    int32 use_alpha;
    int32 depthbuffer_mode;
    int32 cull_mode;
};
RENDER_STATE_GLOBAL render_state;
RENDER_STATE_DEST dest_render_state0;
#define VIEW_MODE__UNKNOWN 0
#define VIEW_MODE__2D 1
#define VIEW_MODE__3D 2
#define VIEW_MODE__RESET 3
#define ALPHA_MODE__UNKNOWN -1
#define ALPHA_MODE__DONT_BLEND 0
#define ALPHA_MODE__BLEND 1
#define TEXTURE_WRAP_MODE__UNKNOWN -1
#define TEXTURE_WRAP_MODE__DONT_WRAP 0
#define TEXTURE_WRAP_MODE__WRAP 1
#define SMOOTH_MODE__UNKNOWN -1
#define SMOOTH_MODE__DONT_SMOOTH 0
#define SMOOTH_MODE__SMOOTH 1
#define PO2_FIX__OFF 0
#define PO2_FIX__EXPANDED 1
#define PO2_FIX__MIPMAPPED 2


#define DEPTHBUFFER_MODE__UNKNOWN -1
#define DEPTHBUFFER_MODE__OFF 0
#define DEPTHBUFFER_MODE__ON 1
#define DEPTHBUFFER_MODE__LOCKED 2
#define DEPTHBUFFER_MODE__CLEAR 3
#define CULL_MODE__UNKNOWN -1
#define CULL_MODE__NONE 0
#define CULL_MODE__CLOCKWISE_ONLY 1
#define CULL_MODE__ANTICLOCKWISE_ONLY 2
/********** Render State **********/

int32 depthbuffer_mode0=DEPTHBUFFER_MODE__ON;
int32 depthbuffer_mode1=DEPTHBUFFER_MODE__ON;

#define INVALID_HARDWARE_HANDLE -1

struct hardware_img_struct{
    int32 w;
    int32 h;
    int32 texture_handle;//if 0, imports from software_pixel_buffer automatically
    int32 dest_context_handle;//used when rendering other images onto this image
    int32 depthbuffer_handle;//generated when 3D commands are called
    int32 pending_commands;//incremented with each command, decremented after command is processed
    int32 remove;//if =1, free immediately after all pending commands are processed
    uint32 *software_pixel_buffer;//if NULL, generates a blank texture
    int32 alpha_disabled;//changed by _BLEND/_DONTBLEND commands
    int32 depthbuffer_mode;//changed by _DEPTHBUFFER
    int32 valid;
    RENDER_STATE_SOURCE source_state;
    RENDER_STATE_DEST dest_state;  
    int32 PO2_w;//if PO2_FIX__EXPANDED/MIPMAPPED, these are the texture size
    int32 PO2_h;
};
list *hardware_img_handles=NULL;

int32 first_hardware_command=0;//only set once
int32 last_hardware_command_added=0;//0 if none exist
int32 last_hardware_command_rendered=0;//0 if all have been processed
int32 next_hardware_command_to_remove=0;//0 if all have been processed

struct hardware_graphics_command_struct{
    int64 order;//which _DISPLAY event to bind the operation to
    int32 next_command;//the handle of the next hardware_graphics_command of the same display-order, of 0 if last
    int64 command;//the command type, actually a set of bit flags
    //Bit 00: Decimal value 000001: _PUTIMAGE
    union{
        int32 option;
        int32 src_img; //MUST be a hardware handle
    };
    union{
        int32 dst_img; //MUST be a hardware handle or 0 for the default 2D rendering context  
        int32 target;
    };
    float src_x1;
    float src_y1;
    float src_x2;
    float src_y2;
    float src_x3;
    float src_y3;
    float dst_x1;
    float dst_y1;
    float dst_z1;
    float dst_x2;
    float dst_y2;
    float dst_z2;
    float dst_x3;
    float dst_y3;
    float dst_z3;
    int32 smooth;//0 or 1 (whether to apply texture filtering)
    int32 cull_mode;
    int32 depthbuffer_mode;
    int32 use_alpha;//0 or 1 (whether to refer to the alpha component of pixel values)
    int32 remove;
};
list *hardware_graphics_command_handles=NULL;
#define HARDWARE_GRAPHICS_COMMAND__PUTIMAGE 1
#define HARDWARE_GRAPHICS_COMMAND__FREEIMAGE_REQUEST 2
#define HARDWARE_GRAPHICS_COMMAND__FREEIMAGE 3
#define HARDWARE_GRAPHICS_COMMAND__MAPTRIANGLE 4
#define HARDWARE_GRAPHICS_COMMAND__MAPTRIANGLE3D 5
#define HARDWARE_GRAPHICS_COMMAND__CLEAR_DEPTHBUFFER 6

int32 SOFTWARE_IMG_HANDLE_MIN=-8388608;
int32 HARDWARE_IMG_HANDLE_OFFSET=-16777216;//added to all hardware image handles to avoid collision
//the lowest integer value a single precision number can exactly represent,
//because users put handles in SINGLEs
#define NEW_HARDWARE_IMG__BUFFER_CONTENT 1
#define NEW_HARDWARE_IMG__DUPLICATE_PROVIDED_BUFFER 2

//R8: libqb/gui.h + gui.cpp replaced by libqb/window_bridge.cpp (Rust window layer)
int32 requestedKeyboardOverlayImage=0;
int32 new_hardware_img(int32 x, int32 y, uint32 *pixels, int32 flags);
void free_hardware_img(int32 handle, int32 caller_id);
void present_frame();
std::recursive_mutex present_mutex;

//these lock values increment
int64 display_lock_request=0;
int64 display_lock_confirmed=0;
int64 display_lock_released=0;

//note: only to be used by user functions, not internal functions
hardware_img_struct *get_hardware_img(int32 handle){
    hardware_img_struct *img;
    if (handle<HARDWARE_IMG_HANDLE_OFFSET||handle>=SOFTWARE_IMG_HANDLE_MIN) return NULL;
    img=(hardware_img_struct*)list_get(hardware_img_handles,handle-HARDWARE_IMG_HANDLE_OFFSET);
    if (img==NULL) return NULL;
    if (!img->valid) return NULL;
    return img;
}
int32 get_hardware_img_index(int32 handle){
    return handle-HARDWARE_IMG_HANDLE_OFFSET;
}
























static uint16 codepage437_to_unicode16[] = {
    0x0020,0x263A,0x263B,0x2665,0x2666,0x2663,0x2660,0x2022,0x25D8,0x25CB,0x25D9,0x2642,0x2640,0x266A,0x266B,0x263C,
    0x25BA,0x25C4,0x2195,0x203C,0x00B6,0x00A7,0x25AC,0x21A8,0x2191,0x2193,0x2192,0x2190,0x221F,0x2194,0x25B2,0x25BC,
    0x0020,0x0021,0x0022,0x0023,0x0024,0x0025,0x0026,0x0027,0x0028,0x0029,0x002A,0x002B,0x002C,0x002D,0x002E,0x002F,
    0x0030,0x0031,0x0032,0x0033,0x0034,0x0035,0x0036,0x0037,0x0038,0x0039,0x003A,0x003B,0x003C,0x003D,0x003E,0x003F,
    0x0040,0x0041,0x0042,0x0043,0x0044,0x0045,0x0046,0x0047,0x0048,0x0049,0x004A,0x004B,0x004C,0x004D,0x004E,0x004F,
    0x0050,0x0051,0x0052,0x0053,0x0054,0x0055,0x0056,0x0057,0x0058,0x0059,0x005A,0x005B,0x005C,0x005D,0x005E,0x005F,
    0x0060,0x0061,0x0062,0x0063,0x0064,0x0065,0x0066,0x0067,0x0068,0x0069,0x006A,0x006B,0x006C,0x006D,0x006E,0x006F,
    0x0070,0x0071,0x0072,0x0073,0x0074,0x0075,0x0076,0x0077,0x0078,0x0079,0x007A,0x007B,0x007C,0x007D,0x007E,0x2302,
    0x00C7,0x00FC,0x00E9,0x00E2,0x00E4,0x00E0,0x00E5,0x00E7,0x00EA,0x00EB,0x00E8,0x00EF,0x00EE,0x00EC,0x00C4,0x00C5,
    0x00C9,0x00E6,0x00C6,0x00F4,0x00F6,0x00F2,0x00FB,0x00F9,0x00FF,0x00D6,0x00DC,0x00A2,0x00A3,0x00A5,0x20A7,0x0192,
    0x00E1,0x00ED,0x00F3,0x00FA,0x00F1,0x00D1,0x00AA,0x00BA,0x00BF,0x2310,0x00AC,0x00BD,0x00BC,0x00A1,0x00AB,0x00BB,
    0x2591,0x2592,0x2593,0x2502,0x2524,0x2561,0x2562,0x2556,0x2555,0x2563,0x2551,0x2557,0x255D,0x255C,0x255B,0x2510,
    0x2514,0x2534,0x252C,0x251C,0x2500,0x253C,0x255E,0x255F,0x255A,0x2554,0x2569,0x2566,0x2560,0x2550,0x256C,0x2567,
    0x2568,0x2564,0x2565,0x2559,0x2558,0x2552,0x2553,0x256B,0x256A,0x2518,0x250C,0x2588,0x2584,0x258C,0x2590,0x2580,
    0x03B1,0x00DF,0x0393,0x03C0,0x03A3,0x03C3,0x00B5,0x03C4,0x03A6,0x0398,0x03A9,0x03B4,0x221E,0x03C6,0x03B5,0x2229,
    0x2261,0x00B1,0x2265,0x2264,0x2320,0x2321,0x00F7,0x2248,0x00B0,0x2219,0x00B7,0x221A,0x207F,0x00B2,0x25A0,0x0020
};


#ifdef QBNex_BACKSLASH_FILESYSTEM
    #include "parts\\video\\font\\ttf\\src.c"
    #else
    #include "parts/video/font/ttf/src.c"
#endif

// Clock reads live in the Rust runtime (crates/qb-runtime/src/systime.rs).
int64 GetTicks(){return qb_get_ticks();}


#define QBK 200000
#define VK 100000
#define UC 1073741824
/* QBK codes:
    200000-200010: Numpad keys with Num-Lock off
    NO_NUMLOCK_KP0=INSERT
    NO_NUMLOCK_KP1=END
    NO_NUMLOCK_KP2=DOWN
    NO_NUMLOCK_KP3=PGDOWN
    NO_NUMLOCK_KP4...
    NO_NUMLOCK_KP5
    NO_NUMLOCK_KP6
    NO_NUMLOCK_KP7
    NO_NUMLOCK_KP8
    NO_NUMLOCK_KP9
    NO_NUMLOCK_KP_PERIOD=DEL
    200011: SCROLL_LOCK_ON
    200012: INSERT_MODE_ON
*/
#define QBK_SCROLL_LOCK_MODE 11
#define QBK_INSERT_MODE 12
#define QBK_CHR0 13
typedef enum {
    QBVK_UNKNOWN    = 0,
    QBVK_FIRST      = 0,
    QBVK_BACKSPACE  = 8,
    QBVK_TAB        = 9,
    QBVK_CLEAR      = 12,
    QBVK_RETURN     = 13,
    QBVK_PAUSE      = 19,
    QBVK_ESCAPE     = 27,
    QBVK_SPACE      = 32,
    QBVK_EXCLAIM    = 33,
    QBVK_QUOTEDBL   = 34,
    QBVK_HASH       = 35,
    QBVK_DOLLAR     = 36,
    QBVK_AMPERSAND  = 38,
    QBVK_QUOTE      = 39,
    QBVK_LEFTPAREN  = 40,
    QBVK_RIGHTPAREN = 41,
    QBVK_ASTERISK   = 42,
    QBVK_PLUS       = 43,
    QBVK_COMMA      = 44,
    QBVK_MINUS      = 45,
    QBVK_PERIOD     = 46,
    QBVK_SLASH      = 47,
    QBVK_0          = 48,
    QBVK_1          = 49,
    QBVK_2          = 50,
    QBVK_3          = 51,
    QBVK_4          = 52,
    QBVK_5          = 53,
    QBVK_6          = 54,
    QBVK_7          = 55,
    QBVK_8          = 56,
    QBVK_9          = 57,
    QBVK_COLON      = 58,
    QBVK_SEMICOLON  = 59,
    QBVK_LESS       = 60,
    QBVK_EQUALS     = 61,
    QBVK_GREATER    = 62,
    QBVK_QUESTION   = 63,
    QBVK_AT         = 64,
    //Skip uppercase letters
    QBVK_LEFTBRACKET= 91,
    QBVK_BACKSLASH  = 92,
    QBVK_RIGHTBRACKET = 93,
    QBVK_CARET      = 94,
    QBVK_UNDERSCORE = 95,
    QBVK_BACKQUOTE  = 96,
    QBVK_a          = 97,
    QBVK_b          = 98,
    QBVK_c          = 99,
    QBVK_d          = 100,
    QBVK_e          = 101,
    QBVK_f          = 102,
    QBVK_g          = 103,
    QBVK_h          = 104,
    QBVK_i          = 105,
    QBVK_j          = 106,
    QBVK_k          = 107,
    QBVK_l          = 108,
    QBVK_m          = 109,
    QBVK_n          = 110,
    QBVK_o          = 111,
    QBVK_p          = 112,
    QBVK_q          = 113,
    QBVK_r          = 114,
    QBVK_s          = 115,
    QBVK_t          = 116,
    QBVK_u          = 117,
    QBVK_v          = 118,
    QBVK_w          = 119,
    QBVK_x          = 120,
    QBVK_y          = 121,
    QBVK_z          = 122,
    QBVK_DELETE     = 127,
    //End of ASCII mapped QBVKs
    //International QBVKs
    QBVK_WORLD_0        = 160,      /* 0xA0 */
    QBVK_WORLD_1        = 161,
    QBVK_WORLD_2        = 162,
    QBVK_WORLD_3        = 163,
    QBVK_WORLD_4        = 164,
    QBVK_WORLD_5        = 165,
    QBVK_WORLD_6        = 166,
    QBVK_WORLD_7        = 167,
    QBVK_WORLD_8        = 168,
    QBVK_WORLD_9        = 169,
    QBVK_WORLD_10       = 170,
    QBVK_WORLD_11       = 171,
    QBVK_WORLD_12       = 172,
    QBVK_WORLD_13       = 173,
    QBVK_WORLD_14       = 174,
    QBVK_WORLD_15       = 175,
    QBVK_WORLD_16       = 176,
    QBVK_WORLD_17       = 177,
    QBVK_WORLD_18       = 178,
    QBVK_WORLD_19       = 179,
    QBVK_WORLD_20       = 180,
    QBVK_WORLD_21       = 181,
    QBVK_WORLD_22       = 182,
    QBVK_WORLD_23       = 183,
    QBVK_WORLD_24       = 184,
    QBVK_WORLD_25       = 185,
    QBVK_WORLD_26       = 186,
    QBVK_WORLD_27       = 187,
    QBVK_WORLD_28       = 188,
    QBVK_WORLD_29       = 189,
    QBVK_WORLD_30       = 190,
    QBVK_WORLD_31       = 191,
    QBVK_WORLD_32       = 192,
    QBVK_WORLD_33       = 193,
    QBVK_WORLD_34       = 194,
    QBVK_WORLD_35       = 195,
    QBVK_WORLD_36       = 196,
    QBVK_WORLD_37       = 197,
    QBVK_WORLD_38       = 198,
    QBVK_WORLD_39       = 199,
    QBVK_WORLD_40       = 200,
    QBVK_WORLD_41       = 201,
    QBVK_WORLD_42       = 202,
    QBVK_WORLD_43       = 203,
    QBVK_WORLD_44       = 204,
    QBVK_WORLD_45       = 205,
    QBVK_WORLD_46       = 206,
    QBVK_WORLD_47       = 207,
    QBVK_WORLD_48       = 208,
    QBVK_WORLD_49       = 209,
    QBVK_WORLD_50       = 210,
    QBVK_WORLD_51       = 211,
    QBVK_WORLD_52       = 212,
    QBVK_WORLD_53       = 213,
    QBVK_WORLD_54       = 214,
    QBVK_WORLD_55       = 215,
    QBVK_WORLD_56       = 216,
    QBVK_WORLD_57       = 217,
    QBVK_WORLD_58       = 218,
    QBVK_WORLD_59       = 219,
    QBVK_WORLD_60       = 220,
    QBVK_WORLD_61       = 221,
    QBVK_WORLD_62       = 222,
    QBVK_WORLD_63       = 223,
    QBVK_WORLD_64       = 224,
    QBVK_WORLD_65       = 225,
    QBVK_WORLD_66       = 226,
    QBVK_WORLD_67       = 227,
    QBVK_WORLD_68       = 228,
    QBVK_WORLD_69       = 229,
    QBVK_WORLD_70       = 230,
    QBVK_WORLD_71       = 231,
    QBVK_WORLD_72       = 232,
    QBVK_WORLD_73       = 233,
    QBVK_WORLD_74       = 234,
    QBVK_WORLD_75       = 235,
    QBVK_WORLD_76       = 236,
    QBVK_WORLD_77       = 237,
    QBVK_WORLD_78       = 238,
    QBVK_WORLD_79       = 239,
    QBVK_WORLD_80       = 240,
    QBVK_WORLD_81       = 241,
    QBVK_WORLD_82       = 242,
    QBVK_WORLD_83       = 243,
    QBVK_WORLD_84       = 244,
    QBVK_WORLD_85       = 245,
    QBVK_WORLD_86       = 246,
    QBVK_WORLD_87       = 247,
    QBVK_WORLD_88       = 248,
    QBVK_WORLD_89       = 249,
    QBVK_WORLD_90       = 250,
    QBVK_WORLD_91       = 251,
    QBVK_WORLD_92       = 252,
    QBVK_WORLD_93       = 253,
    QBVK_WORLD_94       = 254,
    QBVK_WORLD_95       = 255,      /* 0xFF */
    //Numeric keypad
    QBVK_KP0            = 256,
    QBVK_KP1            = 257,
    QBVK_KP2            = 258,
    QBVK_KP3            = 259,
    QBVK_KP4            = 260,
    QBVK_KP5            = 261,
    QBVK_KP6            = 262,
    QBVK_KP7            = 263,
    QBVK_KP8            = 264,
    QBVK_KP9            = 265,
    QBVK_KP_PERIOD      = 266,
    QBVK_KP_DIVIDE      = 267,
    QBVK_KP_MULTIPLY    = 268,
    QBVK_KP_MINUS       = 269,
    QBVK_KP_PLUS        = 270,
    QBVK_KP_ENTER       = 271,
    QBVK_KP_EQUALS      = 272,
    //Arrows + Home/End pad
    QBVK_UP         = 273,
    QBVK_DOWN       = 274,
    QBVK_RIGHT      = 275,
    QBVK_LEFT       = 276,
    QBVK_INSERT     = 277,
    QBVK_HOME       = 278,
    QBVK_END        = 279,
    QBVK_PAGEUP     = 280,
    QBVK_PAGEDOWN   = 281,
    //Function keys
    QBVK_F1         = 282,
    QBVK_F2         = 283,
    QBVK_F3         = 284,
    QBVK_F4         = 285,
    QBVK_F5         = 286,
    QBVK_F6         = 287,
    QBVK_F7         = 288,
    QBVK_F8         = 289,
    QBVK_F9         = 290,
    QBVK_F10        = 291,
    QBVK_F11        = 292,
    QBVK_F12        = 293,
    QBVK_F13        = 294,
    QBVK_F14        = 295,
    QBVK_F15        = 296,
    //Key state modifier keys
    QBVK_NUMLOCK    = 300,
    QBVK_CAPSLOCK   = 301,
    QBVK_SCROLLOCK  = 302,
    //If more modifiers are added, the window defocus code in qbnex_os_event_linux must be altered
    QBVK_RSHIFT     = 303,
    QBVK_LSHIFT     = 304,
    QBVK_RCTRL      = 305,
    QBVK_LCTRL      = 306,
    QBVK_RALT       = 307,
    QBVK_LALT       = 308,
    QBVK_RMETA      = 309,
    QBVK_LMETA      = 310,
    QBVK_LSUPER     = 311,      /* Left "Windows" key */
    QBVK_RSUPER     = 312,      /* Right "Windows" key */
    QBVK_MODE       = 313,      /* "Alt Gr" key */
    QBVK_COMPOSE        = 314,      /* Multi-key compose key */
    //Miscellaneous function keys
    QBVK_HELP       = 315,
    QBVK_PRINT      = 316,
    QBVK_SYSREQ     = 317,
    QBVK_BREAK      = 318,
    QBVK_MENU       = 319,
    QBVK_POWER      = 320,      /* Power Macintosh power key */
    QBVK_EURO       = 321,      /* Some european keyboards */
    QBVK_UNDO       = 322,      /* Atari keyboard has Undo */
    QBVK_LAST
}QBVKs;
//Enumeration of valid key mods (possibly OR'd together)
typedef enum {
    KMOD_NONE  = 0x0000,
    KMOD_LSHIFT= 0x0001,
    KMOD_RSHIFT= 0x0002,
    KMOD_LCTRL = 0x0040,
    KMOD_RCTRL = 0x0080,
    KMOD_LALT  = 0x0100,
    KMOD_RALT  = 0x0200,
    KMOD_LMETA = 0x0400,
    KMOD_RMETA = 0x0800,
    KMOD_NUM   = 0x1000,
    KMOD_CAPS  = 0x2000,
    KMOD_MODE  = 0x4000,
    KMOD_RESERVED = 0x8000
}KMODs;
#define KMOD_CTRL   (KMOD_LCTRL|KMOD_RCTRL)
#define KMOD_SHIFT  (KMOD_LSHIFT|KMOD_RSHIFT)
#define KMOD_ALT    (KMOD_LALT|KMOD_RALT)
#define KMOD_META   (KMOD_LMETA|KMOD_RMETA)



/* Restricted Functionality: (Security focused approach, does not include restricting sound etc)
    
    Block while compiling: (ONLY things that cannot be caught at runtime)
    - $CHECKING:OFF [X]
    - _MEM(x,y) [X]
    - DECLARE LIBRARY [X]
    
    Block at runtime:
    - paths [fixdir]
    - MKDIR [sub_mkdir]
    - SHELL(subs/functions)
    [func__shellhide,
    func__shell,
    sub_shell,
    sub_shell2,
    sub_shell3,
    sub_shell4]
    - RUN "filename" [sub_run]
    - CHAIN [sub_chain]
    - SCREENPRINT [sub__screenprint]
    - SCREENCLICK [sub__screenclick]
    - SCREENIMAGE (returns a blank 1024x768 image)[func__screenimage]
    - ENVIRON [func_environ(num&str), sub_environ]
    
    Reference notes:
    - KILL calls fixdir()
    
    Ports:
    - Client connections are unrestricted
    - Host ports values are either 1 or 2, but the default is port 1 for out of range values
    
*/

//QBNex memory blocks: lock tags, mem_lock_id, mem_lock_tmp and mem_lock_base live in
//the Rust runtime (crates/qb-runtime/src/mem.rs); generated code uses them directly.
static constexpr uint64 MEM_LOCK_INVALID_ID = 1073741821;
#define mem_lock_invalid (*(mem_lock*)qb_mem_lock_invalid())
void new_mem_lock(){ qb_new_mem_lock(); }
void free_mem_lock(mem_lock *lock){ qb_free_mem_lock((qb_mem_lock*)lock); }



/*
    int32 allocated_bytes=0;
    void *malloc2(int x){
    allocated_bytes+=x;
    return malloc(x);
    }
    void *realloc2(void *x, int y){
    allocated_bytes+=y;
    return realloc(x,y);
    }
    void *calloc2(int x, int y){
    allocated_bytes+=(x*y);
    return calloc(x,y);
    }
    #define malloc(x) malloc2(x)
    #define calloc(x,y) calloc2(x,y)
    #define realloc(x,y) realloc2(x,y)
*/

int64 device_event_index=0;
int32 device_mouse_relative=0;

int32 lock_mainloop=0;//0=unlocked, 1=lock requested, 2=locked

int32 lpos=1;
int32 width_lprint=80;

//forward refs
void sub_shell4(qbs*,int32);//_DONTWAIT & _HIDE
int32 func__source();
int32 func_pos(int32 ignore);

double func_timer(double accuracy,int32 passed);
int32 func__newimage(int32 x,int32 y,int32 bpp,int32 passed);
void display();
void validatepage(int32);
void sub__dest(int32);
void sub__source(int32);
int32 func__printwidth(qbs*,int32,int32);
void sub_cls(int32,uint32,int32);
void qbs_print(qbs*,int32);
int32 func__copyimage(int32 i,int32 mode,int32 passed);
int32 func__dest();
int32 func__display();
void qbg_sub_view_print(int32,int32,int32);
qbs *qbs_new(int32,uint8);
qbs *qbs_new_txt(const char*);
qbs *qbs_add(qbs*,qbs*);
qbs *qbs_set(qbs*,qbs*);
void qbg_sub_window(float,float,float,float,int32);
int32 autodisplay=1;
//GFS forward references
int32 gfs_eof_passed(int32 i);
int32 gfs_eof_reached(int32 i);
int64 gfs_getpos(int32 i);
int32 gfs_fileno_valid(int32 f);
int32 gfs_fileno_freefile();//like FREEFILE
void gfs_fileno_use(int32 f, int32 i);
int32 gfs_open(qbs *filename,int32 access,int32 restrictions, int32 how);
int32 gfs_close(int32 i);
int64 gfs_lof(int32 i);
int32 gfs_setpos(int32 i, int64 position);
int32 gfs_write(int32 i,int64 position,uint8 *data,int64 size);
int32 gfs_read(int32 i,int64 position,uint8 *data,int64 size);
int64 gfs_read_bytes();
extern qbs *qbs_str(int64 value);
extern qbs *qbs_str(int32 value);
extern qbs *qbs_str(int16 value);
extern qbs *qbs_str(int8 value);
extern qbs *qbs_str(uint64 value);
extern qbs *qbs_str(uint32 value);
extern qbs *qbs_str(uint16 value);
extern qbs *qbs_str(uint8 value);
extern qbs *qbs_str(float value);
extern qbs *qbs_str(double value);
extern qbs *qbs_str(long double value);
void key_update();
int32 key_display_state=0;
int32 key_display=0;
int32 key_display_redraw=0;

extern int32 device_last;
extern int32 device_max;
extern device_struct *devices;

extern uint8 getDeviceEventButtonValue(device_struct *device, int32 eventIndex, int32 objectIndex);
extern void setDeviceEventButtonValue(device_struct *device, int32 eventIndex, int32 objectIndex, uint8 value);
extern float getDeviceEventAxisValue(device_struct *device, int32 eventIndex, int32 objectIndex);
extern void setDeviceEventAxisValue(device_struct *device, int32 eventIndex, int32 objectIndex, float value);
extern float getDeviceEventWheelValue(device_struct *device, int32 eventIndex, int32 objectIndex);
extern void setDeviceEventWheelValue(device_struct *device, int32 eventIndex, int32 objectIndex, float value);
extern void setupDevice(device_struct *device);
extern int32 createDeviceEvent(device_struct *device);
extern void commitDeviceEvent(device_struct *device);



extern onkey_struct *onkey;//defined in Rust (crates/qb-runtime/src/events.rs)
extern int32 onkey_inprogress;
extern onstrig_struct *onstrig;
extern int32 onstrig_inprogress;

extern uint32 qbevent;

#ifdef DEPENDENCY_DEVICEINPUT
    #include "libqb/gamepad_bridge.cpp" //R8: gilrs backend (qb-runtime gamepad.rs)
#endif

extern int32 console;
extern int32 screen_hide_startup;
extern int32 asserts;
extern int32 vwatch;
//...

extern int64 exit_code;//defined in Rust (crates/qb-runtime/src/program.rs)

int32 console_active=1;
int32 console_child=0;//set if console is only being used by this program
int32 console_image=-1;
int32 screen_hide=0;

//format:[deadkey's symbol in UTF16],[ASCII code of alphabet letter],[resulting UTF16 character]...0
static uint16 deadchar_lookup[] = {
    96,97,224,96,65,192,180,97,225,180,65,193,94,97,226,94,65,194,126,97,227,126,65,195,168,97,228,168,65,196,730,97,229,730,65,197,180,99,263,94,99,265,96,101,232,96,69,200,180,101,233,180,69,201,94,101,234,94,69,202,126,101,7869,168,101,235,168,69,203,180,103,501,94,103,285,94,104,293,168,104,7719,96,105,236,96,73,204,180,105,237,180,73,205,94,105,238,94,73,206,126,105,297,168,105,239,168,73,207,94,106,309,180,107,7729,180,108,314,180,109,7743,96,110,505,180,110,324,126,110,241,126,78,209,96,111,242,96,79,210,180,111,243,180,79,211,94,111,244,94,79,212,126,111,245,126,79,213,168,111,246,168,79,214,180,112,7765,180,114,341,180,115,347,94,115,349,168,116,7831,96,117,249,96,85,217,180,117,250,180,85,218,94,117,251,94,85,219,126,117,361,168,117,252,168,85,220,730,117,367,126,118,7805,96,119,7809,180,119,7811,94,119,373,168,119,7813,730,119,7832,168,120,7821,96,121,7923,180,121,253,180,89,221,94,121,375,126,121,7929,168,121,255,168,89,376,730,121,7833,180,122,378,94,122,7825
    ,0
};

int32 keydown_glyph=0;

int32 convert_unicode(int32 src_fmt,void *src_buf,int32 src_size,int32 dest_fmt,void *dest_buf){
    /*
        important: to ensure enough space is available for the conversion, dest_buf must be at least src_size*4+4 in length
        returns: the number of bytes written to dest_buf
        fmt values:
        1=ASCII(CP437)
        8=UTF8
        16=UTF16
        32=UTF32
    */
    
    static int32 dest_size;
    dest_size=0;
    
    //setup source
    uint8 *src_uint8p=NULL;
    if (src_fmt==1){
        src_uint8p=(uint8*)src_buf;
    }
    uint16 *src_uint16p=NULL;
    if (src_fmt==16){
        src_uint16p=(uint16*)src_buf;
        src_size=src_size-(src_size&1);//cull trailing bytes
    }
    uint32 *src_uint32p=NULL;
    if (src_fmt==32){
        src_uint32p=(uint32*)src_buf;
        src_size=src_size-(src_size&3);//cull trailing bytes
    }
    
    //setup dest
    uint16 *dest_uint16p=NULL;
    if (dest_fmt==16){
        dest_uint16p=(uint16*)dest_buf;
    }
    uint32 *dest_uint32p=NULL;
    if (dest_fmt==32){
        dest_uint32p=(uint32*)dest_buf;
    }
    
    uint32 x;//scalar
    
    while(src_size){
        
        //convert src to scalar UNICODE value 'x'
        
        if (src_fmt==1){//CP437
            x=*src_uint8p++;
            src_size--;
            x=codepage437_to_unicode16[x];
        }
        if (src_fmt==16){//UTF16
            src_size-=2;
            x=*src_uint16p++;
            //note: does not handle surrogate pairs yet
        }
        if (src_fmt==32){//UTF32
            src_size-=4;
            x=*src_uint32p++;
        }
        
        //convert scalar UNICODE value 'x' to dest
        
        if (dest_fmt==16){//UTF16
            *dest_uint16p++=x;
            dest_size+=2;
            //note: does not handle surrogate pairs yet
        }
        if (dest_fmt==32){//UTF32
            *dest_uint32p++=x;
            dest_size+=4;
        }
        
    }//loop
    
    //add NULL terminator (does not change the size in bytes returned)
    if (dest_fmt==16) *dest_uint16p=0;
    if (dest_fmt==32) *dest_uint32p=0;
    
    return dest_size;
}





#ifdef QBNex_WINDOWS
    void showvalue(__int64);
#endif
void sub_beep();

#ifndef DEPENDENCY_AUDIO_OUT
void sub_beep() {
    #ifdef QBNex_WINDOWS
        Beep(800, 120);
    #elif defined(QBNex_MACOSX)
        NSBeep();
    #else
        qb_write_utf8("\a", 1, 0);
    #endif
}

void sub_sound(double frequency, double lengthInClockTicks) {
    (void)frequency;
    (void)lengthInClockTicks;
}

int32 func_play(int32 ignore) {
    (void)ignore;
    return 0;
}

void sub_play(qbs *str) {
    (void)str;
}

void sub__voice(int32 voice) {
    (void)voice;
}

void sub__adsr(double attack, double decay, double sustain, double release, int32 voice, int32 passed) {
    (void)attack;
    (void)decay;
    (void)sustain;
    (void)release;
    (void)voice;
    (void)passed;
}

void sub__wave(qbs *spec, int32 voice, int32 passed) {
    (void)spec;
    (void)voice;
    (void)passed;
}
#endif











int32 func__loadfont(qbs *filename,int32 size,qbs *requirements,int32 passed);



int32 lastfont=48;
std::vector<int32> font(lastfont+1,0);//NULL=unused index
std::vector<int32> fontheight(lastfont+1,0);
std::vector<int32> fontwidth(lastfont+1,0);
std::vector<int32> fontflags(lastfont+1,0);

//keyhit cyclic buffer: R8 moved to qb-runtime keybuf.rs (qb_keyhit_*)
//    keyhit specific internal flags: (stored in high 32-bits)
//    &4294967296->numpad was used




void update_shift_state();




uint32 bindkey=0;

void scancodedown(uint8 scancode);
void scancodeup(uint8 scancode);







/*
    QBNex Mapping of audio control keyboard keys:
    MEDIA_PLAY_PAUSE 0x2200
    MEDIA_STOP 0x2400
    MEDIA_NEXT_TRACK 0x1900
    MEDIA_PREV_TRACK 0x1000
*/

/*
    compatibility upgrade:
    0-255
    num & caps versions are totally ignored
    shift is USUALLY the same as ascii but not always
    0 anywhere else means a key cannot be entered
*/


static const int32 scancode_lookup[]={
    //DESCRIPTION OFFSET  SCANCODE      ASCII      SHIFT       CTRL        ALT        NUM       CAPS SHIFT+CAPS  SHIFT+NUM
    /* ?       */    0 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */    1 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */    2 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */    3 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */    4 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */    5 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */    6 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */    7 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* Bksp    */    8 ,      0x0E,      0x08,      0x08,      0x7F,    0x0E00,         0,         0,         0,         0,
    /* Tab     */    9 ,      0x0F,      0x09,    0x0F00,    0x9400,         0,         0,         0,         0,         0,
    /* ?       */   10 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   11 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   12 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* enter   */   13 ,      0x1C,      0x0D,      0x0D,      0x0A,         0,         0,         0,         0,         0,
    /* ?       */   14 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   15 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   16 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   17 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   18 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   19 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   20 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   21 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   22 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   23 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   24 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   25 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   26 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* Esc     */   27 ,      0x01,      0x1B,      0x1B,      0x1B,         0,         0,         0,         0,         0,
    /* ?       */   28 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   29 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   30 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   31 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* space   */   32 ,      0x39,      0x20,      0x20,      0x20,         0,         0,         0,         0,         0,
    /* 1 !     */   33 ,      0x02,      0x21,      0x21,         0,    0x7800,         0,         0,         0,         0,
    /* \91 \93     */   34 ,      0x28,      0x22,      0x22,         0,    0x2800,         0,         0,         0,         0,
    /* 3 #     */   35 ,      0x04,      0x23,      0x23,         0,    0x7A00,         0,         0,         0,         0,
    /* 4 $     */   36 ,      0x05,      0x24,      0x24,         0,    0x7B00,         0,         0,         0,         0,
    /* 5 %     */   37 ,      0x06,      0x25,      0x25,         0,    0x7C00,         0,         0,         0,         0,
    /* 7 &     */   38 ,      0x08,      0x26,      0x26,         0,    0x7E00,         0,         0,         0,         0,
    /* \91 \93     */   39 ,      0x28,      0x27,      0x27,         0,    0x2800,         0,         0,         0,         0,
    /* 9 (     */   40 ,      0x0A,      0x28,      0x28,         0,    0x8000,         0,         0,         0,         0,
    /* 0 )     */   41 ,      0x0B,      0x29,      0x29,         0,    0x8100,         0,         0,         0,         0,
    /* 8 *     */   42 ,      0x09,      0x2A,      0x2A,         0,    0x7F00,         0,         0,         0,         0,
    /* = +     */   43 ,      0x0D,      0x2B,      0x2B,         0,    0x8300,         0,         0,         0,         0,
    /* , <     */   44 ,      0x33,      0x2C,      0x2C,         0,    0x3300,         0,         0,         0,         0,
    /* - _     */   45 ,      0x0C,      0x2D,      0x2D,      0x1F,    0x8200,         0,         0,         0,         0,
    /* . >     */   46 ,      0x34,      0x2E,      0x2E,         0,    0x3400,         0,         0,         0,         0,
    /* / ?     */   47 ,      0x35,      0x2F,      0x2F,         0,    0x3500,         0,         0,         0,         0,
    /* 0 )     */   48 ,      0x0B,      0x30,      0x30,         0,    0x8100,         0,         0,         0,         0,
    /* 1 !     */   49 ,      0x02,      0x31,      0x31,         0,    0x7800,         0,         0,         0,         0,
    /* 2 @     */   50 ,      0x03,      0x32,      0x32,    0x0300,    0x7900,         0,         0,         0,         0,
    /* 3 #     */   51 ,      0x04,      0x33,      0x33,         0,    0x7A00,         0,         0,         0,         0,
    /* 4 $     */   52 ,      0x05,      0x34,      0x34,         0,    0x7B00,         0,         0,         0,         0,
    /* 5 %     */   53 ,      0x06,      0x35,      0x35,         0,    0x7C00,         0,         0,         0,         0,
    /* 6 ^     */   54 ,      0x07,      0x36,      0x36,      0x1E,    0x7D00,         0,         0,         0,         0,
    /* 7 &     */   55 ,      0x08,      0x37,      0x37,         0,    0x7E00,         0,         0,         0,         0,
    /* 8 *     */   56 ,      0x09,      0x38,      0x38,         0,    0x7F00,         0,         0,         0,         0,
    /* 9 (     */   57 ,      0x0A,      0x39,      0x39,         0,    0x8000,         0,         0,         0,         0,
    /* ; :     */   58 ,      0x27,      0x3A,      0x3A,         0,    0x2700,         0,         0,         0,         0,
    /* ; :     */   59 ,      0x27,      0x3B,      0x3B,         0,    0x2700,         0,         0,         0,         0,
    /* , <     */   60 ,      0x33,      0x3C,      0x3C,         0,    0x3300,         0,         0,         0,         0,
    /* = +     */   61 ,      0x0D,      0x3D,      0x3D,         0,    0x8300,         0,         0,         0,         0,
    /* . >     */   62 ,      0x34,      0x3E,      0x3E,         0,    0x3400,         0,         0,         0,         0,
    /* / ?     */   63 ,      0x35,      0x3F,      0x3F,         0,    0x3500,         0,         0,         0,         0,
    /* 2 @     */   64 ,      0x03,      0x40,      0x40,    0x0300,    0x7900,         0,         0,         0,         0,
    /* A       */   97 ,      0x1E,      0x41,      0x41,      0x01,    0x1E00,         0,         0,         0,         0,
    /* B       */   98 ,      0x30,      0x42,      0x42,      0x02,    0x3000,         0,         0,         0,         0,
    /* C       */   99 ,      0x2E,      0x43,      0x43,      0x03,    0x2E00,         0,         0,         0,         0,
    /* D       */  100 ,      0x20,      0x44,      0x44,      0x04,    0x2000,         0,         0,         0,         0,
    /* E       */  101 ,      0x12,      0x45,      0x45,      0x05,    0x1200,         0,         0,         0,         0,
    /* F       */  102 ,      0x21,      0x46,      0x46,      0x06,    0x2100,         0,         0,         0,         0,
    /* G       */  103 ,      0x22,      0x47,      0x47,      0x07,    0x2200,         0,         0,         0,         0,
    /* H       */  104 ,      0x23,      0x48,      0x48,      0x08,    0x2300,         0,         0,         0,         0,
    /* I       */  105 ,      0x17,      0x49,      0x49,      0x09,    0x1700,         0,         0,         0,         0,
    /* J       */  106 ,      0x24,      0x4A,      0x4A,      0x0A,    0x2400,         0,         0,         0,         0,
    /* K       */  107 ,      0x25,      0x4B,      0x4B,      0x0B,    0x2500,         0,         0,         0,         0,
    /* L       */  108 ,      0x26,      0x4C,      0x4C,      0x0C,    0x2600,         0,         0,         0,         0,
    /* M       */  109 ,      0x32,      0x4D,      0x4D,      0x0D,    0x3200,         0,         0,         0,         0,
    /* N       */  110 ,      0x31,      0x4E,      0x4E,      0x0E,    0x3100,         0,         0,         0,         0,
    /* O       */  111 ,      0x18,      0x4F,      0x4F,      0x0F,    0x1800,         0,         0,         0,         0,
    /* P       */  112 ,      0x19,      0x50,      0x50,      0x10,    0x1900,         0,         0,         0,         0,
    /* Q       */  113 ,      0x10,      0x51,      0x51,      0x11,    0x1000,         0,         0,         0,         0,
    /* R       */  114 ,      0x13,      0x52,      0x52,      0x12,    0x1300,         0,         0,         0,         0,
    /* S       */  115 ,      0x1F,      0x53,      0x53,      0x13,    0x1F00,         0,         0,         0,         0,
    /* T       */  116 ,      0x14,      0x54,      0x54,      0x14,    0x1400,         0,         0,         0,         0,
    /* U       */  117 ,      0x16,      0x55,      0x55,      0x15,    0x1600,         0,         0,         0,         0,
    /* V       */  118 ,      0x2F,      0x56,      0x56,      0x16,    0x2F00,         0,         0,         0,         0,
    /* W       */  119 ,      0x11,      0x57,      0x57,      0x17,    0x1100,         0,         0,         0,         0,
    /* X       */  120 ,      0x2D,      0x58,      0x58,      0x18,    0x2D00,         0,         0,         0,         0,
    /* Y       */  121 ,      0x15,      0x59,      0x59,      0x19,    0x1500,         0,         0,         0,         0,
    /* Z       */  122 ,      0x2C,      0x5A,      0x5A,      0x1A,    0x2C00,         0,         0,         0,         0,
    /* [ {     */   91 ,      0x1A,      0x5B,      0x5B,      0x1B,    0x1A00,         0,         0,         0,         0,
    /* \ |     */   92 ,      0x2B,      0x5C,      0x5C,      0x1C,    0x2B00,         0,         0,         0,         0,
    /* ] }     */   93 ,      0x1B,      0x5D,      0x5D,      0x1D,    0x1B00,         0,         0,         0,         0,
    /* 6 ^     */   94 ,      0x07,      0x5E,      0x5E,      0x1E,    0x7D00,         0,         0,         0,         0,
    /* - _     */   95 ,      0x0C,      0x5F,      0x5F,      0x1F,    0x8200,         0,         0,         0,         0,
    /* ` ~     */   96 ,      0x29,      0x60,      0x60,         0,    0x2900,         0,         0,         0,         0,
    /* A       */   97 ,      0x1E,      0x61,      0x61,      0x01,    0x1E00,         0,         0,         0,         0,
    /* B       */   98 ,      0x30,      0x62,      0x62,      0x02,    0x3000,         0,         0,         0,         0,
    /* C       */   99 ,      0x2E,      0x63,      0x63,      0x03,    0x2E00,         0,         0,         0,         0,
    /* D       */  100 ,      0x20,      0x64,      0x64,      0x04,    0x2000,         0,         0,         0,         0,
    /* E       */  101 ,      0x12,      0x65,      0x65,      0x05,    0x1200,         0,         0,         0,         0,
    /* F       */  102 ,      0x21,      0x66,      0x66,      0x06,    0x2100,         0,         0,         0,         0,
    /* G       */  103 ,      0x22,      0x67,      0x67,      0x07,    0x2200,         0,         0,         0,         0,
    /* H       */  104 ,      0x23,      0x68,      0x68,      0x08,    0x2300,         0,         0,         0,         0,
    /* I       */  105 ,      0x17,      0x69,      0x69,      0x09,    0x1700,         0,         0,         0,         0,
    /* J       */  106 ,      0x24,      0x6A,      0x6A,      0x0A,    0x2400,         0,         0,         0,         0,
    /* K       */  107 ,      0x25,      0x6B,      0x6B,      0x0B,    0x2500,         0,         0,         0,         0,
    /* L       */  108 ,      0x26,      0x6C,      0x6C,      0x0C,    0x2600,         0,         0,         0,         0,
    /* M       */  109 ,      0x32,      0x6D,      0x6D,      0x0D,    0x3200,         0,         0,         0,         0,
    /* N       */  110 ,      0x31,      0x6E,      0x6E,      0x0E,    0x3100,         0,         0,         0,         0,
    /* O       */  111 ,      0x18,      0x6F,      0x6F,      0x0F,    0x1800,         0,         0,         0,         0,
    /* P       */  112 ,      0x19,      0x70,      0x70,      0x10,    0x1900,         0,         0,         0,         0,
    /* Q       */  113 ,      0x10,      0x71,      0x71,      0x11,    0x1000,         0,         0,         0,         0,
    /* R       */  114 ,      0x13,      0x72,      0x72,      0x12,    0x1300,         0,         0,         0,         0,
    /* S       */  115 ,      0x1F,      0x73,      0x73,      0x13,    0x1F00,         0,         0,         0,         0,
    /* T       */  116 ,      0x14,      0x74,      0x74,      0x14,    0x1400,         0,         0,         0,         0,
    /* U       */  117 ,      0x16,      0x75,      0x75,      0x15,    0x1600,         0,         0,         0,         0,
    /* V       */  118 ,      0x2F,      0x76,      0x76,      0x16,    0x2F00,         0,         0,         0,         0,
    /* W       */  119 ,      0x11,      0x77,      0x77,      0x17,    0x1100,         0,         0,         0,         0,
    /* X       */  120 ,      0x2D,      0x78,      0x78,      0x18,    0x2D00,         0,         0,         0,         0,
    /* Y       */  121 ,      0x15,      0x79,      0x79,      0x19,    0x1500,         0,         0,         0,         0,
    /* Z       */  122 ,      0x2C,      0x7A,      0x7A,      0x1A,    0x2C00,         0,         0,         0,         0,
    /* [ {     */  123 ,      0x1A,      0x7B,      0x7B,      0x1B,    0x1A00,         0,         0,         0,         0,
    /* \ |     */  124 ,      0x2B,      0x7C,      0x7C,      0x1C,    0x2B00,         0,         0,         0,         0,
    /* ] }     */  125 ,      0x1B,      0x7D,      0x7D,      0x1D,    0x1B00,         0,         0,         0,         0,
    /* ` ~     */  126 ,      0x29,      0x7E,      0x7E,         0,    0x2900,         0,         0,         0,         0,
    /* ?       */  127 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  128 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  129 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  130 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  131 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  132 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  133 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  134 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  135 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  136 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  137 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  138 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  139 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  140 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  141 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  142 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  143 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  144 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  145 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  146 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  147 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  148 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  149 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  150 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  151 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  152 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  153 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  154 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  155 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  156 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  157 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  158 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  159 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  160 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  161 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  162 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  163 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  164 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  165 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  166 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  167 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  168 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  169 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  170 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  171 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  172 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  173 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  174 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  175 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  176 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  177 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  178 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  179 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  180 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  181 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  182 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  183 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  184 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  185 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  186 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  187 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  188 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  189 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  190 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  191 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  192 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  193 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    
    /* ?       */  194 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  195 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  196 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  197 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  198 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  199 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  200 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  201 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  202 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  203 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  204 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  205 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  206 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  207 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  208 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  209 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  210 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  211 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  212 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  213 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  214 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  215 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  216 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  217 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  218 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  219 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  220 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  221 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  222 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  223 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  224 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  225 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  226 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  227 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  228 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  229 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  230 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  231 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  232 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  233 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  234 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  235 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  236 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  237 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  238 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  239 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  240 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  241 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  242 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  243 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  244 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  245 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  246 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  247 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  248 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  249 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  250 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  251 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  252 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  253 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  254 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  255 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */    0 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */    1 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */    2 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */    3 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */    4 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */    5 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */    6 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */    7 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */    8 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */    9 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   10 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   11 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   12 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   13 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   14 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   15 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   16 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   17 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   18 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   19 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   20 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   21 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   22 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   23 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   24 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   25 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   26 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   27 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   28 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   29 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   30 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   31 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   32 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   33 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   34 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   35 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   36 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   37 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   38 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   39 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   40 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   41 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   42 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   43 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   44 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   45 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   46 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   47 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   48 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   49 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   50 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   51 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   52 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   53 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   54 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   55 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   56 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   57 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   58 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* F1      */   59 ,      0x3B,    0x3B00,    0x5400,    0x5E00,    0x6800,         0,         0,         0,         0,
    /* F2      */   60 ,      0x3C,    0x3C00,    0x5500,    0x5F00,    0x6900,         0,         0,         0,         0,
    /* F3      */   61 ,      0x3D,    0x3D00,    0x5600,    0x6000,    0x6A00,         0,         0,         0,         0,
    /* F4      */   62 ,      0x3E,    0x3E00,    0x5700,    0x6100,    0x6B00,         0,         0,         0,         0,
    /* F5      */   63 ,      0x3F,    0x3F00,    0x5800,    0x6200,    0x6C00,         0,         0,         0,         0,
    /* F6      */   64 ,      0x40,    0x4000,    0x5900,    0x6300,    0x6D00,         0,         0,         0,         0,
    /* F7      */   65 ,      0x41,    0x4100,    0x5A00,    0x6400,    0x6E00,         0,         0,         0,         0,
    /* F8      */   66 ,      0x42,    0x4200,    0x5B00,    0x6500,    0x6F00,         0,         0,         0,         0,
    /* F9      */   67 ,      0x43,    0x4300,    0x5C00,    0x6600,    0x7000,         0,         0,         0,         0,
    /* F10     */   68 ,      0x44,    0x4400,    0x5D00,    0x6700,    0x7100,         0,         0,         0,         0,
    /* ?       */   69 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   70 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* home *X */   71 ,      0x47,    0x4700,    0x4700,    0x7700,    0x9700,         0,         0,         0,         0, //note: X=not on NUMPAD
    /* up *X   */   72 ,      0x48,    0x4800,    0x4800,    0x8D00,    0x9800,         0,         0,         0,         0,
    /* pgup *X */   73 ,      0x49,    0x4900,    0x4900,    0x8400,    0x9900,         0,         0,         0,         0,
    /* ?       */   74 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* left *X */   75 ,      0x4B,    0x4B00,    0x4B00,    0x7300,    0x9B00,         0,         0,         0,         0,
    /* (center)*/   76 ,         0,         0,         0,         0,         0,         0,         0,         0,         0, //note: not used
    /* right *X*/   77 ,      0x4D,    0x4D00,    0x4D00,    0x7400,    0x9D00,         0,         0,         0,         0,
    /* ?       */   78 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* end  *X */   79 ,      0x4F,    0x4F00,    0x4F00,    0x7500,    0x9F00,         0,         0,         0,         0,
    /* down *X */   80 ,      0x50,    0x5000,    0x5000,    0x9100,    0xA000,         0,         0,         0,         0,
    /* pgdn *X */   81 ,      0x51,    0x5100,    0x5100,    0x7600,    0xA100,         0,         0,         0,         0,
    /* ins *X  */   82 ,      0x52,    0x5200,    0x5200,    0x9200,    0xA200,         0,         0,         0,         0,
    /* del *X  */   83 ,      0x53,    0x5300,    0x5300,    0x9300,    0xA300,         0,         0,         0,         0,
    /* ?       */   84 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   85 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   86 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   87 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   88 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   89 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   90 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   91 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   92 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   93 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   94 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   95 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   96 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   97 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   98 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */   99 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  100 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  101 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  102 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  103 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  104 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  105 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  106 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  107 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  108 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  109 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  110 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  111 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  112 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  113 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  114 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  115 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  116 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  117 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  118 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  119 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  120 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  121 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  122 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  123 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  124 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  125 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  126 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  127 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  128 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  129 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  130 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  131 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  132 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* F11     */  133 ,      0x57,    0x8500,    0x8500,    0x8900,    0x8B00,         0,         0,         0,         0,
    /* F12     */  134 ,      0x58,    0x8600,    0x8600,    0x8A00,    0x8C00,         0,         0,         0,         0,
    /* ?       */  135 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  136 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  137 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  138 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  139 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  140 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  141 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  142 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  143 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  144 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  145 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  146 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  147 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  148 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  149 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  150 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  151 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  152 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  153 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  154 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  155 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  156 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  157 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  158 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  159 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  160 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  161 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  162 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  163 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  164 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  165 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  166 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  167 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  168 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  169 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  170 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  171 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  172 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  173 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  174 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  175 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  176 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  177 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  178 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  179 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  180 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  181 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  182 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  183 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  184 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  185 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  186 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  187 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  188 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  189 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  190 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  191 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  192 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  193 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  194 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  195 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  196 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  197 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  198 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  199 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  200 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  201 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  202 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  203 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  204 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  205 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  206 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    
    
    /* ?       */  207 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  208 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  209 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  210 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  211 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  212 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  213 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  214 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  215 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  216 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  217 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  218 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  219 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  220 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  221 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  222 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  223 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  224 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  225 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  226 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  227 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  228 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  229 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  230 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  231 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  232 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  233 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  234 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  235 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  236 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  237 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  238 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  239 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  240 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  241 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  242 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  243 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  244 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  245 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  246 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  247 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  248 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  249 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  250 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  251 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  252 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  253 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  254 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    /* ?       */  255 ,         0,         0,         0,         0,         0,         0,         0,         0,         0,
    //NUMPAD keys in VK order
    /* ins     */  512 ,      0x52,    0x5200,      0x30,    0x9200,         0,      0x30,         0,         0,    0x5200,
    /* end     */  513 ,      0x4F,    0x4F00,      0x31,    0x7500,         0,      0x31,         0,         0,    0x4F00,
    /* down    */  514 ,      0x50,    0x5000,      0x32,    0x9100,         0,      0x32,         0,         0,    0x5000,
    /* pgdn    */  515 ,      0x51,    0x5100,      0x33,    0x7600,         0,      0x33,         0,         0,    0x5100,
    /* left    */  516 ,      0x4B,    0x4B00,      0x34,    0x7300,         0,      0x34,         0,         0,    0x4B00,
    /* center  */  517 ,      0x4C,    0x4C00,      0x35,    0x8F00,         0,      0x35,         0,         0,    0x4C00,
    /* right   */  518 ,      0x4D,    0x4D00,      0x36,    0x7400,         0,      0x36,         0,         0,    0x4D00,
    /* home    */  519 ,      0x47,    0x4700,      0x37,    0x7700,         0,      0x37,         0,         0,    0x4700,
    /* up      */  520 ,      0x48,    0x4800,      0x38,    0x8D00,         0,      0x38,         0,         0,    0x4800,
    /* pgup    */  521 ,      0x49,    0x4900,      0x39,    0x8400,         0,      0x39,         0,         0,    0x4900,
    /* del     */  522 ,      0x53,    0x5300,      0x2E,    0x9300,         0,      0x2E,         0,         0,    0x5300,
    /* divide  */  523 ,      0x35,      0x2F,      0x2F,    0x9500,    0xA400,      0x2F,         0,         0,      0x2F,
    /* mult    */  524 ,      0x37,      0x2A,      0x2A,    0x9600,    0x3700,      0x2A,         0,         0,      0x2A,
    /* -d      */  525 ,      0x4A,      0x2D,      0x2D,    0x8E00,    0x4A00,      0x2D,         0,         0,      0x2D,
    /* +e      */  526 ,      0x4E,      0x2B,      0x2B,    0x9000,    0x4E00,      0x2B,         0,         0,      0x2B,
    /* enter   */  527 ,      0x1C,      0x0D,      0x0D,      0x0A,    0xA600,      0x0D,         0,         0,      0x0D,
    
    /* ctrl    */  000 ,         0,         0,         0,         0,         0,         0,         0,         0,      0x1D,
    /* Lshift  */  000 ,      0x2A,         0,         0,         0,         0,         0,         0,         0,         0,
    /* Rshift  */  000 ,      0x36,         0,         0,         0,         0,         0,         0,         0,         0,
    /* alt     */  000 ,      0x38,         0,         0,         0,         0,         0,         0,         0,         0,
    /* caps    */  000 ,      0x3A,         0,         0,         0,         0,         0,         0,         0,         0,
    /* num     */  000 ,      0x45,         0,         0,         0,         0,         0,         0,         0,         0,
    /* scrl    */  000 ,      0x46,         0,         0,         0,         0,         0,         0,         0,         0,
    /* * PrtSc */  000 ,      0x37,      0x2A,         0,      0x10,         0,      0x2A,      0x2A,         0,         0,
};


void keydown(uint32 x);
void keyup(uint32 x);

uint32 unicode_to_cp437(uint32 x){
    static int32 i;
    for (i=0;i<=255;i++){
        if (x==codepage437_to_unicode16[i]) return i;
    }
    return 0;
}

static int32 qb_img_index(const img_struct *im);
static void qb_init_text_utf8_cells(int32 image_index);
static void qb_clear_text_utf8_cells_range(int32 image_index,int32 first_cell,int32 cell_count);
static int32 qb_text_page_has_utf8_content(int32 image_index);
static void qb_append_utf8_codepoint(std::string &out,uint32 codepoint);
static int32 qb_decode_utf8_codepoint(const uint8 *text,int32 len,uint32 *codepoint);
static int32 qb_is_emoji_like_codepoint(uint32 codepoint);
static int32 qb_is_zero_width_codepoint(uint32 codepoint);
static uint8 qb_unicode_to_screen_byte(uint32 character);
static void qb_store_text_cell_utf8(img_struct *im,int32 cell_index,uint32 character,int32 cell_span);
static int32 qb_append_text_cell_utf8(img_struct *im,int32 cell_index,uint32 character);
#ifdef QBNex_WINDOWS
static int32 qb_render_text_cell_windows(const std::string &utf8,int32 cell_width,int32 cell_height,uint32 foreground_color,uint32 background_color,uint32 *dest,int32 dest_stride);
#else
static int32 qb_render_text_cell_freetype(const std::string &utf8,int32 cell_width,int32 cell_height,uint32 foreground_color,uint32 background_color,uint32 *dest,int32 dest_stride);
#endif

int32 keyheld(uint32 x){
    return qb_keyheld(x);//R8: keybuf.rs (numpad multimapping included)
}
void keyheld_add(uint32 x){
    qb_keyheld_add(x,bindkey);
    bindkey=0;
}
void keyheld_remove(uint32 x){
    qb_keyheld_remove(x);
}
void keyheld_unbind(uint32 x){
    uint32 held;
    if (qb_keyheld_bound_to(x,&held)) keyup(held);
}


void keydown_ascii(uint32 x){
    keydown(x);
}
void keydown_unicode(uint32 x){
    keydown_glyph=1;
    //note: UNICODE 0-127 map directly to ASCII 0-127
    if (x<=127){keydown_ascii(x); return;}
    //note: some UNICODE values map directly to CP437 values found in the extended ASCII set
    static uint32 x2; if (x2=unicode_to_cp437(x)){keydown_ascii(x2); return;}
    //note: full width latin characters will be mapped to their normal width equivalents
    //Wikipedia note: Range U+FF01\96FF5E reproduces the characters of ASCII 21 to 7E as fullwidth forms, that is, a fixed width form used in CJK computing. This is useful for typesetting Latin characters in a CJK  environment. U+FF00 does not correspond to a fullwith ASCII 20 (space character), since that role is already fulfilled by U+3000 "ideographic space."
    if ((x>=0x0000FF01)&&(x<=0x0000FF5E)){keydown_ascii(x-0x0000FF01+0x21); return;}
    if (x==0x3000){keydown_ascii(32); return;}
    x|=UC;
    keydown(x);
}
void keydown_vk(uint32 x){
    keydown(x);
}
void keyup_ascii(uint32 x){
    keyup(x);
}
void keyup_unicode(uint32 x){
    //note: UNICODE 0-127 map directly to ASCII 0-127
    if (x<=127){keyup_ascii(x); return;}
    //note: some UNICODE values map directly to CP437 values found in the extended ASCII set
    static uint32 x2; if (x2=unicode_to_cp437(x)){keyup_ascii(x2); return;}
    //note: full width latin characters will be mapped to their normal width equivalents
    //Wikipedia note: Range U+FF01\96FF5E reproduces the characters of ASCII 21 to 7E as fullwidth forms, that is, a fixed width form used in CJK computing. This is useful for typesetting Latin characters in a CJK  environment. U+FF00 does not correspond to a fullwith ASCII 20 (space character), since that role is already fulfilled by U+3000 "ideographic space."
    if ((x>=0x0000FF01)&&(x<=0x0000FF5E)){keyup_ascii(x-0x0000FF01+0x21); return;}
    if (x==0x3000){keyup_ascii(32); return;}
    x|=UC;
    keyup(x);
}
void keyup_vk(uint32 x){
    keyup(x);
}


extern int32 exit_ok;//defined in Rust (crates/qb-runtime/src/program.rs)

//substitute Windows functionality
#ifndef QBNex_WINDOWS
    //MessageBox defines
    #define IDOK                1
    #define IDCANCEL            2
    #define IDABORT             3
    #define IDRETRY             4
    #define IDIGNORE            5
    #define IDYES               6
    #define IDNO                7
    #define MB_OK                       0x00000000L
    #define MB_OKCANCEL                 0x00000001L
    #define MB_ABORTRETRYIGNORE         0x00000002L
    #define MB_YESNOCANCEL              0x00000003L
    #define MB_YESNO                    0x00000004L
    #define MB_RETRYCANCEL              0x00000005L
    #define MB_SYSTEMMODAL              0x00001000L
    //alternate implementations of MessageBox
    #ifdef QBNex_MACOSX
        int MessageBox(int ignore,char* message, char* header, int type )
        {
            
            
            CFStringRef header_ref      = CFStringCreateWithCString( NULL, header,     kCFStringEncodingASCII );
            CFStringRef message_ref  = CFStringCreateWithCString( NULL, message, kCFStringEncodingASCII );
            CFOptionFlags result;
            if (type&MB_SYSTEMMODAL) type-=MB_SYSTEMMODAL;
            if (type==MB_YESNO){ 
                CFUserNotificationDisplayAlert(
                0, // no timeout
                kCFUserNotificationNoteAlertLevel,
                NULL,
                NULL, 
                NULL,
                header_ref,
                message_ref,
                CFSTR("No"),
                CFSTR("Yes"),
                NULL,
                &result
                );
                CFRelease( header_ref );
                CFRelease( message_ref );
                if( result == kCFUserNotificationDefaultResponse )
                return IDNO;
                else
                return IDYES;
            }
            if (type==MB_OK){ 
                CFUserNotificationDisplayAlert(
                0, // no timeout
                kCFUserNotificationNoteAlertLevel,
                NULL,
                NULL, 
                NULL,
                header_ref,
                message_ref,
                CFSTR("OK"),
                NULL,
                NULL,
                &result
                );
                CFRelease( header_ref );
                CFRelease( message_ref );
                return IDOK;
            }
            return IDCANCEL;
        }
        #else
        int MessageBox(int ignore,char* message,char* title,int type){
            static qbs *s=NULL; if (!s) s=qbs_new(0,0);
            if (type&MB_SYSTEMMODAL) type-=MB_SYSTEMMODAL;
            if (type==MB_YESNO){
                qbs_set(s,qbs_new_txt("xmessage -center -title "));
                qbs_set(s,qbs_add(s,qbs_new_txt("?"))); s->chr[s->len-1]=34;
                qbs_set(s,qbs_add(s,qbs_new_txt(title)));
                qbs_set(s,qbs_add(s,qbs_new_txt("?"))); s->chr[s->len-1]=34;
                qbs_set(s,qbs_add(s,qbs_new_txt(" -buttons Yes:2,No:1 ")));
                qbs_set(s,qbs_add(s,qbs_new_txt("?"))); s->chr[s->len-1]=34;
                qbs_set(s,qbs_add(s,qbs_new_txt(message)));
                qbs_set(s,qbs_add(s,qbs_new_txt("                         ")));
                qbs_set(s,qbs_add(s,qbs_new_txt("?"))); s->chr[s->len-1]=34;
                qbs_set(s,qbs_add(s,qbs_new_txt("?"))); s->chr[s->len-1]=0;
                static int status;
                status=system((char*)s->chr);
                if (-1!=status){
                    static int32 r;
                    r=WEXITSTATUS(status);
                    if (r==2) return IDYES;
                    if (r==1) return IDNO;
                }
                return IDNO;
            }//MB_YESNO
            if (type==MB_OK){
                qbs_set(s,qbs_new_txt("xmessage -center -title "));
                qbs_set(s,qbs_add(s,qbs_new_txt("?"))); s->chr[s->len-1]=34;
                qbs_set(s,qbs_add(s,qbs_new_txt(title)));
                qbs_set(s,qbs_add(s,qbs_new_txt("?"))); s->chr[s->len-1]=34;
                qbs_set(s,qbs_add(s,qbs_new_txt(" -buttons OK:1 ")));
                qbs_set(s,qbs_add(s,qbs_new_txt("?"))); s->chr[s->len-1]=34;
                qbs_set(s,qbs_add(s,qbs_new_txt(message)));
                qbs_set(s,qbs_add(s,qbs_new_txt("                         ")));
                qbs_set(s,qbs_add(s,qbs_new_txt("?"))); s->chr[s->len-1]=34;
                qbs_set(s,qbs_add(s,qbs_new_txt("?"))); s->chr[s->len-1]=0;
                system((char*)s->chr);
                return IDOK;
            }//MB_OK
            return IDCANCEL;
        }
    #endif
    
    void AllocConsole(){
        return;
    }
    void FreeConsole(){
        return;
    }
#endif

int MessageBox2(int ignore,char* message,char* title,int type){

    //headless (QBNEX_HEADLESS / no display): nothing can answer a modal box.
    //The legacy GLUT build got 0 back here (no usable owner window) and went
    //on, so do the same and report the message on stderr instead.
    if (qb_window_headless()){
        fprintf(stderr,"%s: %s\n",title,message);
        return 0;
    }
    #ifdef QBNex_WINDOWS
        return MessageBox(window_handle,message,title,type);
        #else
        return MessageBox(NULL,message,title,type);
    #endif
}



void alert(int32 x){
    static char str[100];
    memset(&str[0],0,100);
    sprintf(str, "%d", x);
    MessageBox(0,&str[0], "Alert", MB_OK );
}

void alert(char *x){
    MessageBox(0,x, "Alert", MB_OK );
}





//vc->project->properties->configuration properties->general->configuration type->application(.exe)
//vc->project->properties->configuration properties->general->configuration type->static library(.lib)


extern void QBMAIN(void *);
void MAIN_LOOP();

#ifdef QBNex_WINDOWS
    extern void QBMAIN_WINDOWS(void *);
    void MAIN_LOOP_WINDOWS(void *);
    #else
    extern void *QBMAIN_LINUX(void *);
    void *MAIN_LOOP_LINUX(void *);
#endif




static void qb_set_background_thread_priority(std::thread &thread){
    #ifdef QBNex_WINDOWS
        SetThreadPriority((HANDLE)thread.native_handle(), THREAD_PRIORITY_NORMAL);
    #else
        (void)thread;
    #endif
}

template <typename Fn>
static void qb_start_detached_thread(Fn &&fn){
    std::thread thread(std::forward<Fn>(fn));
    qb_set_background_thread_priority(thread);
    thread.detach();
}

//directory access defines
#define EPERM           1
#define ENOENT          2
#define ESRCH           3
#define EINTR           4
#define EIO             5
#define ENXIO           6
#define E2BIG           7
#define ENOEXEC         8
#define EBADF           9
#define ECHILD          10
#define EAGAIN          11
#define ENOMEM          12
#define EACCES          13
#define EFAULT          14
#define EBUSY           16
#define EEXIST          17
#define EXDEV           18
#define ENODEV          19
#define ENOTDIR         20
#define EISDIR          21
#define EINVAL          22
#define ENFILE          23
#define EMFILE          24
#define ENOTTY          25
#define EFBIG           27
#define ENOSPC          28
#define ESPIPE          29
#define EROFS           30
#define EMLINK          31
#define EPIPE           32
#define EDOM            33
#define ERANGE          34
#define EDEADLK         36
#define ENAMETOOint32    38
#define ENOLCK          39
#define ENOSYS          40
#define ENOTEMPTY       41
#define EILSEQ          42


int32 lprint=0;//set to 1 during LPRINT operations
int32 lprint_image=0;
double lprint_last=0;//TIMER(0.001) value at last time LPRINT was used
int32 lprint_buffered=0;//set to 1 if content is pending to print
int32 lprint_locked=0;//set to 1 to deny access by QBNex program

/* Generic File System (GFS)
    GFS allows OS specific access whilst still maintaining 'pure' C-based routines for
    multiplatform compatibility. 'Pure' C-based routines may not allow certain functionality,
    such as partial file locking.
    GFS handles/indexes are independent of QBNex handles/indexes to allow for internal files
    to be open but not intefere with the QBNex file handle numbers.
    
    GFS error codes:
    -1 non-specific fail
    -2 invalid handle
    -3 bad/incorrect file mode
    -4 illegal function call (input is out of range)
    -5 file not found (win:2)
    -6 path not found (win:3)
    -7 access/permission denied (win:5,19)
    -8 device unavailable/drive invalid (win:15,21)
    -9 path/file access error
    -10 read past eof
    -11 bad file name
*/

//GFS file table: owned by the Rust runtime (crates/qb-runtime/src/gfs.rs).
//The C++ code that still looks at open files (PRINT # TAB, FIELD strings)
//reads entries through these two views.
typedef qb_gfs_file gfs_file_struct;
static const struct gfs_file_view{
    gfs_file_struct &operator[](int32 i) const { return *qb_gfs_file_at(i); }
} gfs_file{};
static const struct gfs_fileno_view{
    int32 operator[](int32 f) const { return qb_gfs_fileno(f); }
} gfs_fileno{};


static std::vector<uint16> unicode16_buf_storage(1,0);
uint16 *unicode16_buf=unicode16_buf_storage.data();
int32 unicode16_buf_size=static_cast<int32>(sizeof(uint16));
void convert_text_to_utf16(int32 fonthandle,void *buf,int32 size){
    //expand buffer if necessary
    if (unicode16_buf_size<(size*4+4)){
        unicode16_buf_size=size*4+4;
        unicode16_buf_storage.resize((unicode16_buf_size+static_cast<int32>(sizeof(uint16))-1)/static_cast<int32>(sizeof(uint16)),0);
        unicode16_buf=unicode16_buf_storage.data();
    }
    //convert text
    if ((fontflags[fonthandle]&32)&&(fonthandle!=NULL)){//unicode font
        if (size==1) size=4;
        convert_unicode(32,buf,size,16,unicode16_buf);
        }else{
        convert_unicode(1,buf,size,16,unicode16_buf);
    }
}



qbs *unknown_opcode_mess;

extern uint32 ercl;
extern uint32 inclercl;
extern char* includedfilename;

int32 exit_blocked=0;
extern int32 exit_value;//defined in Rust (crates/qb-runtime/src/program.rs)
//1=X-button
//2=CTRL-BREAK
//3=X-button and CTRL-BREAK

//MLP
//int32 qbshlp1=0;

void error(int32 error_number);//for forward references

char *fixdir(qbs *filename){
    //note: changes the slashes in a filename to make it compatible with the OS
    //applied to QB commands: open, bload/bsave, loadfont, loadimage, sndopen/sndplayfile
    static int32 i;

    for (i=0;i<filename->len;i++){
        #ifdef QBNex_WINDOWS
            if (filename->chr[i]==47) filename->chr[i]=92;
            #else
            if (filename->chr[i]==92) filename->chr[i]=47;
        #endif
    }
    return (char*)filename->chr;
}


int32 width8050switch=1;//if set, can automatically switch to WIDTH 80,50 if LOCATE'ing beyond row 26

uint32 pal[256];

extern qbs* nothingstring;

static uint32 sdl_shiftstate=0;

static uint32 sdl_scroll_lock=0;
static uint32 sdl_insert=0;
static uint32 sdl_scroll_lock_prepared=1;
static uint32 sdl_insert_prepared=1;

int32 sub_screen_height_in_characters=-1;//-1=undefined
int32 sub_screen_width_in_characters=-1;//-1=undefined
int32 sub_screen_font=-1;//-1=undefined
int32 sub_screen_keep_page0=0;

int32 key_repeat_on=0;





//QBNex 2D PROTOTYPE 1.0

int32 pages=1;
std::vector<int32> page(1, 0);

#define IMG_BUFFERSIZE 4096
//image registry: img, nimg, nextimg and the pixel/palette buffers are owned by
//Rust (qb-runtime gfx/registry.rs); img moves when the registry grows.
static const int32 qb_img_registry_ready=(qb_img_init(),1);
static std::vector<std::vector<std::string>> qb_text_utf8_cells(IMG_BUFFERSIZE);
static std::vector<std::vector<uint8>> qb_text_utf8_cell_state(IMG_BUFFERSIZE);
void refresh_selected_pages();
static void qb_img_registry_grew(){//img may have moved: resize side tables, re-point pages
    qb_text_utf8_cells.resize(nimg);
    qb_text_utf8_cell_state.resize(nimg);
    refresh_selected_pages();
}
static void qb_set_img_external_offset(int32 index,uint8 *offset){qb_img_set_external_offset(index,offset);}
static void qb_set_img_external_palette(int32 index,uint32 *palette){qb_img_set_external_palette(index,palette);}
static void qb_release_img_pixels(int32 index){qb_img_release_pixels(index);}
static void qb_release_img_palette(int32 index){qb_img_release_palette(index);}
static void qb_release_img_owned_resources(int32 index){qb_img_release_owned(index);}
static void qb_allocate_img_pixels(int32 index,ptrszint bytes){qb_img_alloc_pixels(index,bytes);}
static void qb_allocate_img_palette(int32 index){qb_img_alloc_palette(index);}

static void qb_release_text_utf8_cells(int32 image_index){
    if (image_index < 0) return;
    if (static_cast<size_t>(image_index) < qb_text_utf8_cells.size()){
        std::vector<std::string>().swap(qb_text_utf8_cells[image_index]);
    }
    if (static_cast<size_t>(image_index) < qb_text_utf8_cell_state.size()){
        std::vector<uint8>().swap(qb_text_utf8_cell_state[image_index]);
    }
}

static int32 qb_img_index(const img_struct *im){
    if (!im || !img) return -1;
    return static_cast<int32>(im - img);
}

static void qb_init_text_utf8_cells(int32 image_index){
    if (image_index < 0 || image_index >= nimg) return;
    if (static_cast<size_t>(image_index) >= qb_text_utf8_cells.size()) qb_text_utf8_cells.resize(nimg);
    if (static_cast<size_t>(image_index) >= qb_text_utf8_cell_state.size()) qb_text_utf8_cell_state.resize(nimg);
    if (!img[image_index].text){
        qb_release_text_utf8_cells(image_index);
        return;
    }
    qb_text_utf8_cells[image_index].assign(static_cast<size_t>(img[image_index].width) * img[image_index].height, std::string());
    qb_text_utf8_cell_state[image_index].assign(static_cast<size_t>(img[image_index].width) * img[image_index].height, 0);
}

static void qb_clear_text_utf8_cells_range(int32 image_index,int32 first_cell,int32 cell_count){
    if (image_index < 0 || image_index >= nimg) return;
    if (static_cast<size_t>(image_index) >= qb_text_utf8_cells.size()) return;
    std::vector<std::string> &cells=qb_text_utf8_cells[image_index];
    std::vector<uint8> &cell_state=qb_text_utf8_cell_state[image_index];
    if (cells.empty() || cell_count <= 0) return;
    if (first_cell < 0) first_cell=0;
    int32 end_cell=first_cell+cell_count;
    if (end_cell > static_cast<int32>(cells.size())) end_cell=static_cast<int32>(cells.size());
    for (int32 i=first_cell;i<end_cell;i++){
        cells[i].clear();
        cell_state[i]=0;
    }
}

static int32 qb_text_page_has_utf8_content(int32 image_index){
    if (image_index < 0 || image_index >= nimg) return 0;
    if (static_cast<size_t>(image_index) >= qb_text_utf8_cells.size()) return 0;
    std::vector<std::string> &cells=qb_text_utf8_cells[image_index];
    for (size_t i=0;i<cells.size();i++){
        if (!cells[i].empty()) return -1;
    }
    return 0;
}

static void qb_append_utf8_codepoint(std::string &out,uint32 codepoint){
    if (codepoint > 0x10FFFF) codepoint=0xFFFD;
    if (codepoint <= 0x7F){
        out.push_back(static_cast<char>(codepoint));
        return;
    }
    if (codepoint <= 0x7FF){
        out.push_back(static_cast<char>(0xC0 | (codepoint >> 6)));
        out.push_back(static_cast<char>(0x80 | (codepoint & 0x3F)));
        return;
    }
    if (codepoint <= 0xFFFF){
        out.push_back(static_cast<char>(0xE0 | (codepoint >> 12)));
        out.push_back(static_cast<char>(0x80 | ((codepoint >> 6) & 0x3F)));
        out.push_back(static_cast<char>(0x80 | (codepoint & 0x3F)));
        return;
    }
    out.push_back(static_cast<char>(0xF0 | (codepoint >> 18)));
    out.push_back(static_cast<char>(0x80 | ((codepoint >> 12) & 0x3F)));
    out.push_back(static_cast<char>(0x80 | ((codepoint >> 6) & 0x3F)));
    out.push_back(static_cast<char>(0x80 | (codepoint & 0x3F)));
}

static int32 qb_decode_utf8_codepoint(const uint8 *text,int32 len,uint32 *codepoint){
    if (!text || len <= 0 || !codepoint) return 0;

    const uint8 b0=text[0];
    if (b0 < 0x80){
        *codepoint=b0;
        return 1;
    }

    if ((b0 >= 0xC2) && (b0 <= 0xDF) && len >= 2){
        const uint8 b1=text[1];
        if ((b1 & 0xC0) == 0x80){
            *codepoint=((b0 & 0x1F) << 6) | (b1 & 0x3F);
            return 2;
        }
    }

    if ((b0 >= 0xE0) && (b0 <= 0xEF) && len >= 3){
        const uint8 b1=text[1];
        const uint8 b2=text[2];
        if (((b1 & 0xC0) == 0x80) && ((b2 & 0xC0) == 0x80)){
            uint32 value=((b0 & 0x0F) << 12) | ((b1 & 0x3F) << 6) | (b2 & 0x3F);
            if ((value >= 0x800) && !(value >= 0xD800 && value <= 0xDFFF)){
                if (!((b0 == 0xE0) && (b1 < 0xA0)) && !((b0 == 0xED) && (b1 >= 0xA0))){
                    *codepoint=value;
                    return 3;
                }
            }
        }
    }

    if ((b0 >= 0xF0) && (b0 <= 0xF4) && len >= 4){
        const uint8 b1=text[1];
        const uint8 b2=text[2];
        const uint8 b3=text[3];
        if (((b1 & 0xC0) == 0x80) && ((b2 & 0xC0) == 0x80) && ((b3 & 0xC0) == 0x80)){
            uint32 value=((b0 & 0x07) << 18) | ((b1 & 0x3F) << 12) | ((b2 & 0x3F) << 6) | (b3 & 0x3F);
            if ((value >= 0x10000) && (value <= 0x10FFFF)){
                if (!((b0 == 0xF0) && (b1 < 0x90)) && !((b0 == 0xF4) && (b1 > 0x8F))){
                    *codepoint=value;
                    return 4;
                }
            }
        }
    }

    *codepoint=codepage437_to_unicode16[b0];
    return 1;
}

static int32 qb_is_emoji_like_codepoint(uint32 codepoint){
    if ((codepoint >= 0x1F1E6 && codepoint <= 0x1F1FF) ||
        (codepoint >= 0x1F300 && codepoint <= 0x1FAFF) ||
        (codepoint >= 0x2600 && codepoint <= 0x27BF) ||
        (codepoint >= 0x2300 && codepoint <= 0x23FF) ||
        codepoint == 0x20E3) return -1;
    return 0;
}

static int32 qb_is_zero_width_codepoint(uint32 codepoint){
    if ((codepoint >= 0x0300 && codepoint <= 0x036F) ||
        (codepoint >= 0x1AB0 && codepoint <= 0x1AFF) ||
        (codepoint >= 0x1DC0 && codepoint <= 0x1DFF) ||
        (codepoint >= 0x20D0 && codepoint <= 0x20FF) ||
        (codepoint >= 0xFE20 && codepoint <= 0xFE2F) ||
        (codepoint >= 0xFE00 && codepoint <= 0xFE0F) ||
        (codepoint >= 0xE0100 && codepoint <= 0xE01EF)) return -1;
    if (codepoint == 0x0E31 || codepoint == 0x200C || codepoint == 0x200D || codepoint == 0xFEFF) return -1;
    if ((codepoint >= 0x0E34 && codepoint <= 0x0E3A) || (codepoint >= 0x0E47 && codepoint <= 0x0E4E)) return -1;
    return 0;
}

static int32 qb_is_wide_codepoint(uint32 codepoint){
    if (qb_is_emoji_like_codepoint(codepoint)) return -1;
    if ((codepoint >= 0x1100 && codepoint <= 0x115F) ||
        (codepoint >= 0x2329 && codepoint <= 0x232A) ||
        (codepoint >= 0x2E80 && codepoint <= 0xA4CF && codepoint != 0x303F) ||
        (codepoint >= 0xAC00 && codepoint <= 0xD7A3) ||
        (codepoint >= 0xF900 && codepoint <= 0xFAFF) ||
        (codepoint >= 0xFE10 && codepoint <= 0xFE19) ||
        (codepoint >= 0xFE30 && codepoint <= 0xFE6F) ||
        (codepoint >= 0xFF01 && codepoint <= 0xFF60) ||
        (codepoint >= 0xFFE0 && codepoint <= 0xFFE6)) return -1;
    return 0;
}

static uint8 qb_unicode_to_screen_byte(uint32 character){
    if (character <= 0x7F) return static_cast<uint8>(character);
    const uint32 mapped=unicode_to_cp437(character);
    if (mapped <= 0xFF) return static_cast<uint8>(mapped);
    return '?';
}

static void qb_normalize_text_cell_state(int32 image_index,int32 cell_index){
    if (image_index < 0 || image_index >= nimg) return;
    if (static_cast<size_t>(image_index) >= qb_text_utf8_cells.size()) return;
    std::vector<std::string> &cells=qb_text_utf8_cells[image_index];
    std::vector<uint8> &cell_state=qb_text_utf8_cell_state[image_index];
    if (cell_index < 0 || cell_index >= static_cast<int32>(cells.size())) return;

    if (cell_state[cell_index] == 3 && cell_index > 0 && cell_state[cell_index - 1] == 2){
        cells[cell_index - 1].clear();
        cell_state[cell_index - 1]=0;
    }
    if (cell_state[cell_index] == 2 && cell_index + 1 < static_cast<int32>(cells.size()) && cell_state[cell_index + 1] == 3){
        cells[cell_index + 1].clear();
        cell_state[cell_index + 1]=0;
    }

    cells[cell_index].clear();
    cell_state[cell_index]=0;
}

static int32 qb_text_cluster_start(int32 image_index,int32 cell_index){
    if (image_index < 0 || image_index >= nimg) return cell_index;
    if (static_cast<size_t>(image_index) >= qb_text_utf8_cells.size()) return cell_index;
    std::vector<uint8> &cell_state=qb_text_utf8_cell_state[image_index];
    if (cell_index < 0 || cell_index >= static_cast<int32>(cell_state.size())) return cell_index;
    if (cell_state[cell_index] == 3 && cell_index > 0) return cell_index - 1;
    return cell_index;
}

static int32 qb_text_cell_span(int32 image_index,int32 cell_index){
    if (image_index < 0 || image_index >= nimg) return 1;
    if (static_cast<size_t>(image_index) >= qb_text_utf8_cells.size()) return 1;
    std::vector<uint8> &cell_state=qb_text_utf8_cell_state[image_index];
    if (cell_index < 0 || cell_index >= static_cast<int32>(cell_state.size())) return 1;
    if (cell_state[cell_index] == 2) return 2;
    if (cell_state[cell_index] == 3) return 0;
    return 1;
}

static void qb_store_text_cell_utf8(img_struct *im,int32 cell_index,uint32 character,int32 cell_span){
    const int32 image_index=qb_img_index(im);
    if (image_index < 0 || image_index >= nimg) return;
    if (static_cast<size_t>(image_index) >= qb_text_utf8_cells.size()) return;
    std::vector<std::string> &cells=qb_text_utf8_cells[image_index];
    std::vector<uint8> &cell_state=qb_text_utf8_cell_state[image_index];
    if (cell_index < 0 || cell_index >= static_cast<int32>(cells.size())) return;

    qb_normalize_text_cell_state(image_index,cell_index);
    if (cell_span > 1 && cell_index + 1 < static_cast<int32>(cells.size())) qb_normalize_text_cell_state(image_index,cell_index + 1);

    if (character <= 0x7F || unicode_to_cp437(character)){
        cell_state[cell_index]=0;
        return;
    }

    qb_append_utf8_codepoint(cells[cell_index],character);
    cell_state[cell_index]=(cell_span > 1) ? 2 : 1;
    if (cell_span > 1 && cell_index + 1 < static_cast<int32>(cells.size())){
        cells[cell_index + 1].clear();
        cell_state[cell_index + 1]=3;
    }
}

static int32 qb_append_text_cell_utf8(img_struct *im,int32 cell_index,uint32 character){
    const int32 image_index=qb_img_index(im);
    if (image_index < 0 || image_index >= nimg) return 0;
    if (static_cast<size_t>(image_index) >= qb_text_utf8_cells.size()) return 0;
    std::vector<std::string> &cells=qb_text_utf8_cells[image_index];
    std::vector<uint8> &cell_state=qb_text_utf8_cell_state[image_index];
    cell_index=qb_text_cluster_start(image_index,cell_index);
    if (cell_index < 0 || cell_index >= static_cast<int32>(cells.size())) return 0;
    if (cells[cell_index].empty()){
        const uint8 raw_character=im->offset[cell_index<<1];
        if (raw_character != 32) qb_append_utf8_codepoint(cells[cell_index],codepage437_to_unicode16[raw_character]);
        cell_state[cell_index]=1;
    }
    qb_append_utf8_codepoint(cells[cell_index],character);
    return 1;
}

#ifdef QBNex_WINDOWS
static HDC qb_textmode_gdi_dc=NULL;
static HBITMAP qb_textmode_gdi_bitmap=NULL;
static HGDIOBJ qb_textmode_gdi_previous_bitmap=NULL;
static uint32 *qb_textmode_gdi_pixels=NULL;
static int32 qb_textmode_gdi_bitmap_width=0;
static int32 qb_textmode_gdi_bitmap_height=0;
static HFONT qb_textmode_gdi_fonts[5][3]={{NULL}};

static int32 qb_get_textmode_script_slot(uint32 codepoint){
    if (qb_is_emoji_like_codepoint(codepoint)) return 4;
    if (codepoint >= 0x0E00 && codepoint <= 0x0E7F) return 1;
    if ((codepoint >= 0x3040 && codepoint <= 0x30FF) || (codepoint >= 0x3000 && codepoint <= 0x303F) || (codepoint >= 0x4E00 && codepoint <= 0x9FFF)) return 2;
    if ((codepoint >= 0x1100 && codepoint <= 0x11FF) || (codepoint >= 0x3130 && codepoint <= 0x318F) || (codepoint >= 0xAC00 && codepoint <= 0xD7AF)) return 3;
    return 0;
}

static int32 qb_get_textmode_height_slot(int32 cell_height){
    if (cell_height <= 8) return 0;
    if (cell_height <= 14) return 1;
    return 2;
}

static int32 qb_get_textmode_requested_height(int32 script_slot,int32 cell_height){
    int32 requested_height=cell_height;
    if ((script_slot == 2 || script_slot == 3 || script_slot == 4) && requested_height > 8) requested_height--;
    if ((script_slot == 1 || script_slot == 4) && requested_height > 14) requested_height--;
    if (requested_height < 1) requested_height=1;
    return requested_height;
}

static HFONT qb_get_textmode_gdi_font(uint32 codepoint,int32 cell_width,int32 cell_height){
    static const wchar_t *faces[5]={L"Segoe UI",L"Leelawadee UI",L"MS Gothic",L"Malgun Gothic",L"Segoe UI Emoji"};
    const int32 script_slot=qb_get_textmode_script_slot(codepoint);
    const int32 height_slot=qb_get_textmode_height_slot(cell_height);
    HFONT *slot=&qb_textmode_gdi_fonts[script_slot][height_slot];
    const int32 requested_height=qb_get_textmode_requested_height(script_slot,cell_height);

    if (*slot) return *slot;

    *slot=CreateFontW(
        -requested_height,
        0,
        0,
        0,
        FW_NORMAL,
        FALSE,
        FALSE,
        FALSE,
        DEFAULT_CHARSET,
        OUT_OUTLINE_PRECIS,
        CLIP_DEFAULT_PRECIS,
        CLEARTYPE_QUALITY,
        DEFAULT_PITCH | FF_DONTCARE,
        faces[script_slot]
    );
    return *slot;
}

static int32 qb_prepare_textmode_gdi_bitmap(int32 cell_width,int32 cell_height){
    if (!qb_textmode_gdi_dc) qb_textmode_gdi_dc=CreateCompatibleDC(NULL);
    if (!qb_textmode_gdi_dc) return 0;
    if (cell_width <= qb_textmode_gdi_bitmap_width && cell_height <= qb_textmode_gdi_bitmap_height && qb_textmode_gdi_pixels) return -1;

    if (qb_textmode_gdi_bitmap){
        SelectObject(qb_textmode_gdi_dc,qb_textmode_gdi_previous_bitmap);
        DeleteObject(qb_textmode_gdi_bitmap);
        qb_textmode_gdi_bitmap=NULL;
        qb_textmode_gdi_pixels=NULL;
        qb_textmode_gdi_bitmap_width=0;
        qb_textmode_gdi_bitmap_height=0;
    }

    BITMAPINFO bitmap_info;
    ZeroMemory(&bitmap_info,sizeof(bitmap_info));
    bitmap_info.bmiHeader.biSize=sizeof(bitmap_info.bmiHeader);
    bitmap_info.bmiHeader.biWidth=cell_width;
    bitmap_info.bmiHeader.biHeight=-cell_height;
    bitmap_info.bmiHeader.biPlanes=1;
    bitmap_info.bmiHeader.biBitCount=32;
    bitmap_info.bmiHeader.biCompression=BI_RGB;

    void *bitmap_pixels=NULL;
    qb_textmode_gdi_bitmap=CreateDIBSection(qb_textmode_gdi_dc,&bitmap_info,DIB_RGB_COLORS,&bitmap_pixels,NULL,0);
    if (!qb_textmode_gdi_bitmap || !bitmap_pixels) return 0;

    qb_textmode_gdi_previous_bitmap=SelectObject(qb_textmode_gdi_dc,qb_textmode_gdi_bitmap);
    qb_textmode_gdi_pixels=(uint32*)bitmap_pixels;
    qb_textmode_gdi_bitmap_width=cell_width;
    qb_textmode_gdi_bitmap_height=cell_height;
    SetTextAlign(qb_textmode_gdi_dc,TA_LEFT | TA_TOP);
    return -1;
}

static int32 qb_render_text_cell_windows(const std::string &utf8,int32 cell_width,int32 cell_height,uint32 foreground_color,uint32 background_color,uint32 *dest,int32 dest_stride){
    if (utf8.empty() || !dest || cell_width <= 0 || cell_height <= 0) return 0;
    if (!qb_prepare_textmode_gdi_bitmap(cell_width,cell_height)) return 0;

    std::wstring wide_text;
    if (!qb_multibyte_to_wide(utf8.data(),static_cast<int32>(utf8.size()),CP_UTF8,0,wide_text) || wide_text.empty()) return 0;

    RECT draw_rect={0,0,cell_width,cell_height};
    HBRUSH background_brush=CreateSolidBrush(RGB((background_color>>16)&255,(background_color>>8)&255,background_color&255));
    FillRect(qb_textmode_gdi_dc,&draw_rect,background_brush);
    DeleteObject(background_brush);

    HFONT font_handle=qb_get_textmode_gdi_font(static_cast<uint32>(wide_text[0]),cell_width,cell_height);
    HGDIOBJ previous_font=NULL;
    if (font_handle) previous_font=SelectObject(qb_textmode_gdi_dc,font_handle);
    SetTextColor(qb_textmode_gdi_dc,RGB((foreground_color>>16)&255,(foreground_color>>8)&255,foreground_color&255));
    SetBkMode(qb_textmode_gdi_dc,TRANSPARENT);
    DrawTextW(qb_textmode_gdi_dc,wide_text.c_str(),static_cast<int32>(wide_text.size()),&draw_rect,DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
    if (previous_font) SelectObject(qb_textmode_gdi_dc,previous_font);

    for (int32 y=0;y<cell_height;y++){
        uint32 *src=qb_textmode_gdi_pixels+y*qb_textmode_gdi_bitmap_width;
        uint32 *dst=dest+y*dest_stride;
        for (int32 x=0;x<cell_width;x++) dst[x]=src[x];
    }

    return -1;
}
#else
static int32 qb_get_textmode_script_slot(uint32 codepoint){
    if (qb_is_emoji_like_codepoint(codepoint)) return 4;
    if (codepoint >= 0x0E00 && codepoint <= 0x0E7F) return 1;
    if ((codepoint >= 0x3040 && codepoint <= 0x30FF) || (codepoint >= 0x3000 && codepoint <= 0x303F) || (codepoint >= 0x4E00 && codepoint <= 0x9FFF)) return 2;
    if ((codepoint >= 0x1100 && codepoint <= 0x11FF) || (codepoint >= 0x3130 && codepoint <= 0x318F) || (codepoint >= 0xAC00 && codepoint <= 0xD7AF)) return 3;
    return 0;
}

static int32 qb_get_textmode_height_slot(int32 cell_height){
    if (cell_height <= 8) return 0;
    if (cell_height <= 14) return 1;
    return 2;
}

static int32 qb_get_textmode_requested_height(int32 script_slot,int32 cell_height){
    int32 requested_height=cell_height;
    if ((script_slot == 2 || script_slot == 3 || script_slot == 4) && requested_height > 8) requested_height--;
    if ((script_slot == 1 || script_slot == 4) && requested_height > 14) requested_height--;
    if (requested_height < 1) requested_height=1;
    return requested_height;
}

static int32 qb_textmode_freetype_fonts[5][3]={{0}};
static uint8 qb_textmode_freetype_attempted[5][3]={{0}};

static void qb_fill_text_cell(uint32 *dest,int32 dest_stride,int32 cell_width,int32 cell_height,uint32 color){
    int32 x,y;
    for (y=0;y<cell_height;y++){
        uint32 *row=dest+y*dest_stride;
        for (x=0;x<cell_width;x++) row[x]=color;
    }
}

static uint32 qb_blend_textmode_color(uint32 background_color,uint32 foreground_color,uint8 alpha){
    if (!alpha) return background_color;
    if (alpha == 255) return foreground_color;

    const uint32 inverse_alpha=255-alpha;
    const uint32 r=((((background_color>>16)&255)*inverse_alpha)+(((foreground_color>>16)&255)*alpha)+127)/255;
    const uint32 g=((((background_color>>8)&255)*inverse_alpha)+(((foreground_color>>8)&255)*alpha)+127)/255;
    const uint32 b=((((background_color)&255)*inverse_alpha)+(((foreground_color)&255)*alpha)+127)/255;
    return (r<<16)|(g<<8)|b;
}

static int32 qb_read_binary_file(const std::string &path,std::vector<uint8> &content){
    std::ifstream file(path.c_str(),std::ios::binary);
    std::streamoff size;

    content.clear();
    if (!file) return 0;

    file.seekg(0,std::ios::end);
    size=file.tellg();
    if (size <= 0) return 0;
    file.seekg(0,std::ios::beg);

    content.resize(static_cast<size_t>(size));
    if (!file.read(reinterpret_cast<char*>(&content[0]),static_cast<std::streamsize>(content.size()))) return 0;
    return -1;
}

static int32 qb_decode_utf8_text(const std::string &utf8,std::vector<uint32> &codepoints){
    int32 offset=0;
    uint32 codepoint=0;
    int32 consumed=0;

    codepoints.clear();
    while (offset < static_cast<int32>(utf8.size())){
        consumed=qb_decode_utf8_codepoint(reinterpret_cast<const uint8*>(utf8.data())+offset,static_cast<int32>(utf8.size())-offset,&codepoint);
        if (consumed <= 0) return 0;
        codepoints.push_back(codepoint);
        offset+=consumed;
    }

    return codepoints.empty() ? 0 : -1;
}

static uint32 qb_get_primary_text_cell_codepoint(const std::vector<uint32> &codepoints){
    size_t i;
    for (i=0;i<codepoints.size();i++){
        if (!qb_is_zero_width_codepoint(codepoints[i])) return codepoints[i];
    }
    if (!codepoints.empty()) return codepoints[0];
    return 0;
}

#ifdef QBNex_MACOSX
static std::string qb_resolve_macos_font_path(const char *font_name){
    std::string path;
    CFStringRef font_name_ref;
    CTFontRef font;
    CFTypeRef url_value;

    if (!font_name || !font_name[0]) return path;

    font_name_ref=CFStringCreateWithCString(kCFAllocatorDefault,font_name,kCFStringEncodingUTF8);
    if (!font_name_ref) return path;

    font=CTFontCreateWithName(font_name_ref,12.0,NULL);
    CFRelease(font_name_ref);
    if (!font) return path;

    url_value=CTFontCopyAttribute(font,kCTFontURLAttribute);
    if (url_value && CFGetTypeID(url_value)==CFURLGetTypeID()){
        char buffer[4096];
        if (CFURLGetFileSystemRepresentation((CFURLRef)url_value,true,reinterpret_cast<UInt8*>(buffer),sizeof(buffer))){
            path.assign(buffer);
        }
    }

    if (url_value) CFRelease(url_value);
    CFRelease(font);
    return path;
}

static const char* const* qb_get_macos_textmode_font_candidates(int32 script_slot){
    static const char* const default_faces[]={"Helvetica","Arial Unicode MS","Menlo",NULL};
    static const char* const thai_faces[]={"Thonburi","Sukhumvit Set","Arial Unicode MS",NULL};
    static const char* const cjk_faces[]={"Hiragino Sans","PingFang SC","Arial Unicode MS",NULL};
    static const char* const korean_faces[]={"Apple SD Gothic Neo","AppleGothic","Arial Unicode MS",NULL};
    static const char* const emoji_faces[]={"Apple Symbols","Apple Color Emoji","Arial Unicode MS",NULL};

    if (script_slot == 1) return thai_faces;
    if (script_slot == 2) return cjk_faces;
    if (script_slot == 3) return korean_faces;
    if (script_slot == 4) return emoji_faces;
    return default_faces;
}

static int32 qb_try_load_textmode_font_candidates(const char* const* candidates,int32 requested_height){
    std::vector<uint8> content;
    int32 i=0;

    if (!candidates) return 0;

    while (candidates[i]){
        std::string font_path=qb_resolve_macos_font_path(candidates[i]);
        if (!font_path.empty() && qb_read_binary_file(font_path,content)){
            int32 handle=FontLoad(&content[0],static_cast<int32>(content.size()),requested_height,-1,32);
            if (handle) return handle;
        }
        i++;
    }

    return 0;
}
#else
static const char* const* qb_get_linux_textmode_font_candidates(int32 script_slot){
    static const char* const default_paths[]={
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/liberation2/LiberationSans-Regular.ttf",
        "/usr/share/fonts/liberation/LiberationSans-Regular.ttf",
        NULL
    };
    static const char* const thai_paths[]={
        "/usr/share/fonts/truetype/noto/NotoSansThai-Regular.ttf",
        "/usr/share/fonts/opentype/noto/NotoSansThai-Regular.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        NULL
    };
    static const char* const cjk_paths[]={
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJKjp-Regular.otf",
        "/usr/share/fonts/truetype/noto/NotoSansJP-Regular.otf",
        NULL
    };
    static const char* const korean_paths[]={
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJKkr-Regular.otf",
        "/usr/share/fonts/truetype/noto/NotoSansKR-Regular.ttf",
        NULL
    };
    static const char* const emoji_paths[]={
        "/usr/share/fonts/truetype/noto/NotoSansSymbols2-Regular.ttf",
        "/usr/share/fonts/truetype/noto/NotoSansSymbols-Regular.ttf",
        "/usr/share/fonts/truetype/noto/NotoEmoji-Regular.ttf",
        "/usr/share/fonts/truetype/ancient-scripts/Symbola_hint.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        NULL
    };

    if (script_slot == 1) return thai_paths;
    if (script_slot == 2) return cjk_paths;
    if (script_slot == 3) return korean_paths;
    if (script_slot == 4) return emoji_paths;
    return default_paths;
}

static int32 qb_try_load_textmode_font_candidates(const char* const* candidates,int32 requested_height){
    std::vector<uint8> content;
    int32 i=0;

    if (!candidates) return 0;

    while (candidates[i]){
        if (qb_read_binary_file(candidates[i],content)){
            int32 handle=FontLoad(&content[0],static_cast<int32>(content.size()),requested_height,-1,32);
            if (handle) return handle;
        }
        i++;
    }

    return 0;
}
#endif

static int32 qb_get_textmode_freetype_font(uint32 codepoint,int32 cell_height){
    const int32 script_slot=qb_get_textmode_script_slot(codepoint);
    const int32 height_slot=qb_get_textmode_height_slot(cell_height);
    const int32 requested_height=qb_get_textmode_requested_height(script_slot,cell_height);
    int32 handle=0;

    if (qb_textmode_freetype_fonts[script_slot][height_slot]) return qb_textmode_freetype_fonts[script_slot][height_slot];
    if (qb_textmode_freetype_attempted[script_slot][height_slot]) return 0;

    qb_textmode_freetype_attempted[script_slot][height_slot]=1;

#ifdef QBNex_MACOSX
    handle=qb_try_load_textmode_font_candidates(qb_get_macos_textmode_font_candidates(script_slot),requested_height);
    if (!handle && script_slot != 0) handle=qb_try_load_textmode_font_candidates(qb_get_macos_textmode_font_candidates(0),requested_height);
#else
    handle=qb_try_load_textmode_font_candidates(qb_get_linux_textmode_font_candidates(script_slot),requested_height);
    if (!handle && script_slot != 0) handle=qb_try_load_textmode_font_candidates(qb_get_linux_textmode_font_candidates(0),requested_height);
#endif

    qb_textmode_freetype_fonts[script_slot][height_slot]=handle;
    return handle;
}

static int32 qb_render_text_cell_freetype(const std::string &utf8,int32 cell_width,int32 cell_height,uint32 foreground_color,uint32 background_color,uint32 *dest,int32 dest_stride){
    std::vector<uint32> codepoints;
    uint8 *alpha_bitmap=NULL;
    int32 bitmap_width=0,bitmap_height=0,pre_x=0,post_x=0;
    int32 font_handle=0;
    uint32 primary_codepoint=0;
    int32 min_x,max_x,min_y,max_y;
    int32 src_x,src_y,copy_width,copy_height,dest_x,dest_y;
    int32 x,y;

    if (utf8.empty() || !dest || cell_width <= 0 || cell_height <= 0) return 0;
    if (!qb_decode_utf8_text(utf8,codepoints)) return 0;

    primary_codepoint=qb_get_primary_text_cell_codepoint(codepoints);
    font_handle=qb_get_textmode_freetype_font(primary_codepoint,cell_height);
    if (!font_handle) return 0;

    if (!FontRenderTextUTF32(font_handle,&codepoints[0],static_cast<int32>(codepoints.size()),0,&alpha_bitmap,&bitmap_width,&bitmap_height,&pre_x,&post_x) || !alpha_bitmap) return 0;

    min_x=bitmap_width;
    min_y=bitmap_height;
    max_x=-1;
    max_y=-1;
    for (y=0;y<bitmap_height;y++){
        for (x=0;x<bitmap_width;x++){
            if (alpha_bitmap[x+y*bitmap_width]){
                if (x < min_x) min_x=x;
                if (y < min_y) min_y=y;
                if (x > max_x) max_x=x;
                if (y > max_y) max_y=y;
            }
        }
    }

    if (max_x < min_x || max_y < min_y){
        free(alpha_bitmap);
        return 0;
    }

    qb_fill_text_cell(dest,dest_stride,cell_width,cell_height,background_color);

    src_x=min_x;
    src_y=min_y;
    copy_width=max_x-min_x+1;
    copy_height=max_y-min_y+1;
    dest_x=(cell_width-copy_width)/2;
    dest_y=(cell_height-copy_height)/2;

    if (dest_x < 0){
        src_x+=-dest_x;
        copy_width+=dest_x;
        dest_x=0;
    }
    if (dest_y < 0){
        src_y+=-dest_y;
        copy_height+=dest_y;
        dest_y=0;
    }
    if ((dest_x+copy_width) > cell_width) copy_width=cell_width-dest_x;
    if ((dest_y+copy_height) > cell_height) copy_height=cell_height-dest_y;

    if (copy_width > 0 && copy_height > 0){
        for (y=0;y<copy_height;y++){
            uint32 *dest_row=dest+(dest_y+y)*dest_stride+dest_x;
            uint8 *src_row=alpha_bitmap+(src_y+y)*bitmap_width+src_x;
            for (x=0;x<copy_width;x++){
                dest_row[x]=qb_blend_textmode_color(background_color,foreground_color,src_row[x]);
            }
        }
    }

    free(alpha_bitmap);
    return -1;
}
#endif

//freed indexes are recycled by the Rust registry (was std::vector<uint32> fimg)

uint8 *cblend=NULL;
uint8 *ablend=NULL;
uint8 *ablend127;
uint8 *ablend128;
//software blend tables (Rust, built on first use)
void init_blend(){
    qb_gfx_blend_tables((const uint8**)&cblend,(const uint8**)&ablend);
    ablend127=ablend+(127<<8);
    ablend128=ablend+(128<<8);
}


uint32 display_page_index=0;
uint32 write_page_index=0;
uint32 read_page_index=0;
//use of non-indexed forms assumes valid indexes (may not be suitable for all commands)
img_struct *write_page=NULL;
img_struct *read_page=NULL;
img_struct *display_page=NULL;
uint32 *display_surface_offset=0;

void ensure_page_entries(int32 required_pages){
    if (required_pages <= pages) return;
    page.resize(required_pages, 0);
    pages = required_pages;
}

void select_display_page(uint32 index){
    display_page_index=index;
    display_page=&img[index];
}

void select_write_page(uint32 index){
    write_page_index=index;
    write_page=&img[index];
}

void select_read_page(uint32 index){
    read_page_index=index;
    read_page=&img[index];
}

void select_write_and_read_page(uint32 index){
    select_write_page(index);
    select_read_page(index);
}

void select_all_pages(uint32 index){
    select_display_page(index);
    select_write_and_read_page(index);
}

void refresh_selected_pages(){
    select_display_page(display_page_index);
    select_write_page(write_page_index);
    select_read_page(read_page_index);
}

inline int32 resolve_page_or_image_index(int32 handle,int32 &resolved_index){
    if (handle>=0){
        validatepage(handle);
        resolved_index=page[handle];
        return -1;
    }
    handle=-handle;
    if (handle>=nextimg){error(258); return 0;}
    if (!img[handle].valid){error(258); return 0;}
    resolved_index=handle;
    return -1;
}

inline img_struct* resolve_page_or_image_ptr(int32 handle,int32 &resolved_index){
    if (!resolve_page_or_image_index(handle,resolved_index)) return NULL;
    return &img[resolved_index];
}

//next_img_capacity: Rust (qb-runtime gfx/registry.rs)

int32 grow_img_registry(int32 required_slots){
    if (required_slots <= nimg) return -1;
    if (int32 e=qb_img_grow(required_slots)){error(e); return 0;}
    qb_img_registry_grew();
    return -1;
}

void restorepalette(img_struct* im){qb_gfx_restorepalette(im);}




void pset(int32 x,int32 y,uint32 col){qb_gfx_pset(write_page,x,y,col);}



/*
    img_struct *img=(img_struct*)malloc(1024*sizeof(img_struct));
    uint32 nimg=1024;
    uint32 nextimg=0;//-1=none have been assigned
    
    uint32 *freeimg=(uint32*)malloc(1024*4);//a list to recover freed indexes
    uint32 nfreeimg=1024;
    uint32 lastfreeimg=-1;//-1=no freed indexes exist
*/

//returns an index to free img structure
uint32 newimg(){
    int32 old_capacity=nimg,e=0;
    uint32 i=qb_img_new(&e);
    if (e) error(e);
    if (nimg!=old_capacity) qb_img_registry_grew();
    return i;
}

int32 freeimg(uint32 i){
    //returns: 0=failed, 1=success
    if (i>=nimg) return 0;
    if (!img[i].valid) return 0;
    if (img[i].lock_id){
        free_mem_lock((mem_lock*)img[i].lock_offset);//untag
    }
    qb_release_text_utf8_cells(i);
    for (size_t page_index = 0; page_index < page.size(); ++page_index){
        if (page[page_index] == static_cast<int32>(i)) page[page_index] = 0;
    }
    qb_img_free(i);//buffers, struct reset, free list (Rust)
    return 1;
}


void imgrevert(int32 i){qb_imgrevert(i);}//text surfaces keep their cells (as before)

int32 imgframe(uint8 *o,int32 x,int32 y,int32 bpp){
    int32 old_capacity=nimg,e=0;
    int32 i=qb_imgframe(o,x,y,bpp,&e);
    if (e) error(e);
    if (nimg!=old_capacity) qb_img_registry_grew();
    if (!i) return 0;
    if (!bpp) qb_init_text_utf8_cells(i); else qb_release_text_utf8_cells(i);
    return i;
}

void sub__freeimage(int32 i,int32 passed);//forward ref

int32 imgnew(int32 x,int32 y,int32 bpp){
    int32 old_capacity=nimg,e=0;
    int32 i=qb_imgnew(x,y,bpp,&e);
    if (e) error(e);
    if (nimg!=old_capacity) qb_img_registry_grew();
    if (!i) return 0;
    if (bpp==32&&!cblend) init_blend();
    if (!bpp) qb_init_text_utf8_cells(i); else qb_release_text_utf8_cells(i);
    return i;
}

void sub__font(int32 f,int32 i,int32 passed);//foward def

void flush_old_hardware_commands(){
    static int32 old_command;
    static int32 command_to_remove;
    static hardware_graphics_command_struct* last_rendered_hgc;
    static hardware_graphics_command_struct* old_hgc;
    static hardware_graphics_command_struct* next_hgc;
    
    if (next_hardware_command_to_remove&&last_hardware_command_rendered){
        
        last_rendered_hgc=(hardware_graphics_command_struct*)list_get(hardware_graphics_command_handles,last_hardware_command_rendered);
        
        old_command=next_hardware_command_to_remove;
        old_hgc=(hardware_graphics_command_struct*)list_get(hardware_graphics_command_handles,old_command);
        
        remove_next_hgc:
        
        if (old_hgc->next_command==0) goto cant_remove;
        next_hgc=(hardware_graphics_command_struct*)list_get(hardware_graphics_command_handles,old_hgc->next_command);
        if (next_hgc->order>=last_rendered_hgc->order) goto cant_remove;
        
        
        command_to_remove=old_command;
        
        if (old_hgc->command==HARDWARE_GRAPHICS_COMMAND__FREEIMAGE_REQUEST){    
            static hardware_img_struct *himg;
            himg=(hardware_img_struct*)list_get(hardware_img_handles,old_hgc->src_img);
            //alert("HARDWARE_GRAPHICS_COMMAND__FREEIMAGE_REQUEST");
            //alert(old_hgc->src_img);
            //add command to free image
            //create new command handle & structure
            int32 hgch=list_add(hardware_graphics_command_handles);
            hardware_graphics_command_struct* hgc=(hardware_graphics_command_struct*)list_get(hardware_graphics_command_handles,hgch);
            hgc->remove=0;
            //set command values
            hgc->command=HARDWARE_GRAPHICS_COMMAND__FREEIMAGE;
            hgc->src_img=old_hgc->src_img;
            //queue the command
            hgc->next_command=0;
            hgc->order=display_frame_order_next;
            if (last_hardware_command_added){
                hardware_graphics_command_struct* hgc2=(hardware_graphics_command_struct*)list_get(hardware_graphics_command_handles,last_hardware_command_added);
                hgc2->next_command=hgch;
            }
            last_hardware_command_added=hgch;
            if (first_hardware_command==0) first_hardware_command=hgch;    
        }
        
        
        
        old_command=old_hgc->next_command;
        next_hardware_command_to_remove=old_command;
        old_hgc=(hardware_graphics_command_struct*)list_get(hardware_graphics_command_handles,old_command);
        list_remove(hardware_graphics_command_handles,command_to_remove);
        
        goto remove_next_hgc;
        
        cant_remove:;
        
        
    }//next_hardware_command_to_remove&&last_hardware_command_rendered
}//flush_old_hardware_commands


void sub__putimage(double f_dx1,double f_dy1,double f_dx2,double f_dy2,int32 src,int32 dst,double f_sx1,double f_sy1,double f_sx2,double f_sy2,int32 passed){
    
    /* 
        Format & passed bits: (needs updating)
        [[{STEP}](?,?)[-[{STEP}](?,?)]][,[?][,[?][,[[{STEP}](?,?)[-[{STEP}](?,?)]][,{_SMOOTH}]]]]
        2?     1              4?       8                                 512      128
    */
    
    static int32 w,h,sskip,dskip,x,y,xx,yy,z,x2,y2,dbpp,sbpp;
    static img_struct *s,*d;
    static uint32 *soff32,*doff32,col,clearcol,destcol;
    static uint8 *soff,*doff;
    static uint8 *cp;
    static int32 xdir,ydir,no_stretch,no_clip,no_reverse,flip,mirror;
    static double mx,my,fx,fy,fsx1,fsy1,fsx2,fsy2,dv,dv2;
    static int32 sx1,sy1,sx2,sy2,dx1,dy1,dx2,dy2;
    static int32 sw,sh,dw,dh;
    static uint32 *pal;
    static uint32 *ulp;
    
    no_stretch=0; no_clip=0; no_reverse=1;
    flip=0; mirror=0;
    
    static int32 use_hardware;
    static img_struct s_emu,d_emu;//used to emulate a source/dest image for calculation purposes
    use_hardware=0;
    
    //is source a hardware handle?
    hardware_img_struct* dst_himg=NULL;
    hardware_img_struct* src_himg=NULL;
    if (src){
        src_himg=(hardware_img_struct*)list_get(hardware_img_handles,src-HARDWARE_IMG_HANDLE_OFFSET);
        if (src_himg!=NULL){//source is hardware image
            src-=HARDWARE_IMG_HANDLE_OFFSET;
            
            flush_old_hardware_commands();
            
            s_emu.width=src_himg->w;
            s_emu.height=src_himg->h;
            s_emu.clipping_or_scaling=0;
            s_emu.alpha_disabled=src_himg->alpha_disabled;
            s=&s_emu;
            
            //check dst      
            if (dst<0){
                dst_himg=(hardware_img_struct*)list_get(hardware_img_handles,dst-HARDWARE_IMG_HANDLE_OFFSET);
                if (dst_himg==NULL){error(258); return;}
                dst-=HARDWARE_IMG_HANDLE_OFFSET;
                
                d_emu.width=dst_himg->w;
                d_emu.height=dst_himg->h;
                d_emu.clipping_or_scaling=0;
                d_emu.alpha_disabled=dst_himg->alpha_disabled;
                d=&d_emu;
                
                }else{
                if (dst>1) {error(5); return;}
                dst=-dst;
                
                d=display_page;//use parameters from display page
                
            }
            
            sbpp=4; dbpp=4;
            use_hardware=1;
            
            goto resolve_coordinates;
            
        }//source is hardware image
    }//src passed
    
    
    
    if (passed&8){//src
        //validate
        if (src>=0){
            validatepage(src); s=&img[page[src]];
            }else{
            src=-src;
            if (src>=nextimg){error(258); return;}
            s=&img[src];
            if (!s->valid){error(258); return;}
        }
        }else{
        s=read_page;
    }//src
    if (s->text){error(5); return;}
    sbpp=s->bytes_per_pixel;
    
    if (passed&32){//dst
        //validate
        if (dst>=0){
            validatepage(dst); d=&img[page[dst]];
            }else{
            dst=-dst;
            if (dst>=nextimg){error(258); return;}
            d=&img[dst];
            if (!d->valid){error(258); return;}
        }
        }else{
        d=write_page;
    }//dst
    if (d->text){error(5); return;}
    dbpp=d->bytes_per_pixel;
    if ((sbpp==4)&&(dbpp==1)){error(5); return;}
    if (s==d){
        //{error(5); return;}//cannot put source onto itself!
        int32 temphandle=func__copyimage(dst,NULL,0);
        passed=passed|8; //make certain we set the flag TO LET QBNex know we're passing a handle to the temp image
        sub__putimage(f_dx1, f_dy1, f_dx2, f_dy2,temphandle, dst, f_sx1, f_sy1, f_sx2, f_sy2, passed);
        sub__freeimage(temphandle,1);
        return;
    }
    
    
    resolve_coordinates:
    //coordinates, clipping, stretching and the software copy: Rust (qb-runtime gfx/putimage.rs)
    {
        qb_put_coords pc={f_dx1,f_dy1,f_dx2,f_dy2,f_sx1,f_sy1,f_sx2,f_sy2};
        if (!use_hardware){
            if (int32 e=qb_gfx_putimage(s,d,&pc,passed,NULL)) error(e);
            return;
        }
        int32 hw[8];
        if (int32 e=qb_gfx_putimage(s,d,&pc,passed,hw)){error(e); return;}
        sx1=hw[0]; sy1=hw[1]; sx2=hw[2]; sy2=hw[3]; dx1=hw[4]; dy1=hw[5]; dx2=hw[6]; dy2=hw[7];
    }
    {
        int32 hgch=list_add(hardware_graphics_command_handles);
        hardware_graphics_command_struct* hgc=(hardware_graphics_command_struct*)list_get(hardware_graphics_command_handles,hgch);
        
        hgc->remove=0;
        
        //set command values
        hgc->command=HARDWARE_GRAPHICS_COMMAND__PUTIMAGE;
        
        hgc->src_img=src;
        hgc->src_x1=sx1;
        hgc->src_y1=sy1;
        hgc->src_x2=sx2;
        hgc->src_y2=sy2;
        
        hgc->dst_img=dst;
        hgc->dst_x1=dx1;
        hgc->dst_y1=dy1;
        hgc->dst_x2=dx2;
        hgc->dst_y2=dy2;
        
        hgc->smooth=0;//unless specified, no filtering will be applied
        if (passed&128) hgc->smooth=1;
        
        hgc->use_alpha=1;
        if (s->alpha_disabled) hgc->use_alpha=0;
        //only consider dest alpha setting if it is a hardware image    
        if (dst_himg){
            if (d->alpha_disabled) hgc->use_alpha=0;
        }
        
        //queue the command
        hgc->next_command=0;
        hgc->order=display_frame_order_next;
        
        if (last_hardware_command_added){
            hardware_graphics_command_struct* hgc2=(hardware_graphics_command_struct*)list_get(hardware_graphics_command_handles,last_hardware_command_added);
            hgc2->next_command=hgch;
        }
        last_hardware_command_added=hgch;
        if (first_hardware_command==0) first_hardware_command=hgch;
        
        return;
        
    }
}//_putimage


int32 selectfont(int32 f,img_struct *im){
    im->font=f;
    im->cursor_x=1; im->cursor_y=1;
    im->top_row=1;
    if (im->compatible_mode) im->bottom_row=im->height/fontheight[f]; else im->bottom_row=im->height;
    im->bottom_row--; if (im->bottom_row<=0) im->bottom_row=1;
    return 1;//success
}


int32 nmodes=0;
int32 anymode=0;

float x_scale=1,y_scale=1;
int32 x_offset=0,y_offset=0;
int32 x_limit=0,y_limit=0;



int32 x_monitor=0,y_monitor=0;


int32 conversion_required=0;
static std::vector<uint32> conversion_layer_storage(2,0);
uint32 *conversion_layer=conversion_layer_storage.data();



void call_int(int32 i);

uint32 frame=0;


extern uint8 cmem[1114099];//16*65535+65535+3 (enough for highest referencable dword in conv memory)


int32 mouse_hideshow_called=0;

struct mouse_message{
    int16 x;
    int16 y;
    uint32 buttons;
    int16 movementx;
    int16 movementy;
};

/*
    mouse_message mouse_messages[65536];//a circular buffer of mouse messages
    int32 last_mouse_message=0;
    int32 current_mouse_message=0;
*/

//Mouse message queue system
//--------------------------
struct mouse_message_queue_struct{
    mouse_message *queue;
    int32 lastIndex;
    int32 current;
    int32 first;
    int32 last;
    int32 child;
    int32 parent;
};
list *mouse_message_queue_handles=NULL;
int32 mouse_message_queue_first; //the first queue to populate from input source
int32 mouse_message_queue_default;//the default queue (for int33h and default _MOUSEINPUT operations)
static std::unordered_map<mouse_message_queue_struct*, std::vector<mouse_message>> mouse_message_queue_storage;

static void initialize_mouse_message_queue(mouse_message_queue_struct *queue, int32 last_index){
    auto &buffer=mouse_message_queue_storage[queue];
    buffer.assign(static_cast<size_t>(last_index)+1, mouse_message{});
    queue->queue=buffer.data();
    queue->lastIndex=last_index;
}






//x86 Virtual CMEM emulation
//Note: x86 CPU emulation is still experimental and is not available in QBNex yet.
struct cpu_struct{
    //al,ah,ax,eax (unsigned & signed)
    union{
        struct{
            union{
                uint8 al;
                int8 al_signed;
            };
            union{
                uint8 ah;
                int8 ah_signed;
            };
        };
        uint16 ax;
        int16 ax_signed;
        uint32 eax;
        int32 eax_signed;
    };
    //bl,bh,bx,ebx (unsigned & signed)
    union{
        struct{
            union{
                uint8 bl;
                int8 bl_signed;
            };
            union{
                uint8 bh;
                int8 bh_signed;
            };
        };
        uint16 bx;
        int16 bx_signed;
        uint32 ebx;
        int32 ebx_signed;
    };
    //cl,ch,cx,ecx (unsigned & signed)
    union{
        struct{
            union{
                uint8 cl;
                int8 cl_signed;
            };
            union{
                uint8 ch;
                int8 ch_signed;
            };
        };
        uint16 cx;
        int16 cx_signed;
        uint32 ecx;
        int32 ecx_signed;
    };
    //dl,dh,dx,edx (unsigned & signed)
    union{
        struct{
            union{
                uint8 dl;
                int8 dl_signed;
            };
            union{
                uint8 dh;
                int8 dh_signed;
            };
        };
        uint16 dx;
        int16 dx_signed;
        uint32 edx;
        int32 edx_signed;
    };
    //si,esi (unsigned & signed)
    union{
        uint16 si;
        int16 si_signed;
        uint32 esi;
        int32 esi_signed;
    };
    //di,edi (unsigned & signed)
    union{
        uint16 di;
        int16 di_signed;
        uint32 edi;
        int32 edi_signed;
    };
    //bp,ebp (unsigned & signed)
    union{
        uint16 bp;
        int16 bp_signed;
        uint32 ebp;
        int32 ebp_signed;
    };
    //sp,esp (unsigned & signed)
    union{
        uint16 sp;
        int16 sp_signed;
        uint32 esp;
        int32 esp_signed;
    };
    //cs,ss,ds,es,fs,gs (unsigned & signed)
    union{
        uint16 cs;
        uint16 cs_signed;
    };
    union{
        uint16 ss;
        uint16 ss_signed;
    };
    union{
        uint16 ds;
        uint16 ds_signed;
    };
    union{
        uint16 es;
        uint16 es_signed;
    };
    union{
        uint16 fs;
        uint16 fs_signed;
    };
    union{
        uint16 gs;
        uint16 gs_signed;
    };
    //ip,eip (unsigned & signed)
    union{
        uint16 ip;
        uint16 ip_signed;
        uint32 eip;
        uint32 eip_signed;
    };
    //flags
    uint8 overflow_flag;
    uint8 direction_flag;
    uint8 interrupt_flag;
    uint8 trap_flag;
    uint8 sign_flag;
    uint8 zero_flag;
    uint8 auxiliary_flag;
    uint8 parity_flag;
    uint8 carry_flag;
};
cpu_struct cpu;

uint8 *ip;
uint8 *seg;//default segment (DS unless overridden)
uint8 *seg_bp;//the segment bp will be accessed from (SS unless overridden)

uint8 *reg8[8];
uint16 *reg16[8];
uint32 *reg32[8];
uint16 *segreg[8];

int32 a32;
int32 b32;//size of data to read/write in bits is 32


uint32 sib(){
    static uint32 i;//sib byte
    i=*ip++;
    switch(i>>6){
        case 0:
        return *reg32[i&7]+*reg32[i>>3&7];
        break;
        case 1:
        return *reg32[i&7]+(*reg32[i>>3&7]<<1);
        break;
        case 2:
        return *reg32[i&7]+(*reg32[i>>3&7]<<2);
        break;
        case 3:
        return *reg32[i&7]+(*reg32[i>>3&7]<<3);
        break;
    }
}

uint32 sib_mod0(){
    //Note: Called when top 2 bits of rm byte before sib byte were 0, base register is ignored
    //      and replaced with an int32 following the sib byte
    static uint32 i;//sib byte
    i=*ip++;
    if ((i&7)==5){
        switch(i>>6){
            case 0:
            return (*(uint32*)((ip+=4)-4))+*reg32[i>>3&7];
            break;
            case 1:
            return (*(uint32*)((ip+=4)-4))+(*reg32[i>>3&7]<<1);
            break;
            case 2:
            return (*(uint32*)((ip+=4)-4))+(*reg32[i>>3&7]<<2);
            break;
            case 3:
            return (*(uint32*)((ip+=4)-4))+(*reg32[i>>3&7]<<3);
            break;
        }
    }
    switch(i>>6){
        case 0:
        return *reg32[i&7]+*reg32[i>>3&7];
        break;
        case 1:
        return *reg32[i&7]+(*reg32[i>>3&7]<<1);
        break;
        case 2:
        return *reg32[i&7]+(*reg32[i>>3&7]<<2);
        break;
        case 3:
        return *reg32[i&7]+(*reg32[i>>3&7]<<3);
        break;
    }
}

uint8 *rm8(){
    static uint32 i;//r/m byte
    i=*ip++;
    switch(i>>6){
        case 3:
        return reg8[i&7];
        break;
        case 0:
        if (a32){
            switch(i&7){
                case 0: return seg+cpu.ax; break;
                case 1: return seg+cpu.cx; break;
                case 2: return seg+cpu.dx; break;
                case 3: return seg+cpu.bx; break;
                case 4: return seg+(uint16)sib_mod0(); break;
                case 5: return seg+(*(uint16*)((ip+=4)-4)); break;
                case 6: return seg+cpu.si; break;
                case 7: return seg+cpu.di; break;
            }
            }else{
            switch(i&7){
                case 0: return seg+((uint16)(cpu.bx+cpu.si)); break;
                case 1: return seg+((uint16)(cpu.bx+cpu.di)); break;
                case 2: return seg_bp+((uint16)(cpu.bp+cpu.si)); break;
                case 3: return seg_bp+((uint16)(cpu.bp+cpu.di)); break;
                case 4: return seg+cpu.si; break;
                case 5: return seg+cpu.di; break;
                case 6: return seg+(*(uint16*)((ip+=2)-2)); break;
                case 7: return seg+cpu.bx; break;
            }
        }
        break;
        case 1:
        if (a32){
            switch(i&7){
                case 0: return seg+((uint16)(cpu.eax+*(int8*)ip++)); break;
                case 1: return seg+((uint16)(cpu.ecx+*(int8*)ip++)); break;
                case 2: return seg+((uint16)(cpu.edx+*(int8*)ip++)); break;
                case 3: return seg+((uint16)(cpu.ebx+*(int8*)ip++)); break;
                case 4: i=sib(); return seg+((uint16)(i+*(int8*)ip++)); break;
                case 5: return seg_bp+((uint16)(cpu.ebp+*(int8*)ip++)); break;
                case 6: return seg+((uint16)(cpu.esi+*(int8*)ip++)); break;
                case 7: return seg+((uint16)(cpu.edi+*(int8*)ip++)); break;
            }
            }else{
            switch(i&7){
                case 0: return seg+((uint16)(cpu.bx+cpu.si+*(int8*)ip++)); break;
                case 1: return seg+((uint16)(cpu.bx+cpu.di+*(int8*)ip++)); break;
                case 2: return seg_bp+((uint16)(cpu.bp+cpu.si+*(int8*)ip++)); break;
                case 3: return seg_bp+((uint16)(cpu.bp+cpu.di+*(int8*)ip++)); break;
                case 4: return seg+((uint16)(cpu.si+*(int8*)ip++)); break;
                case 5: return seg+((uint16)(cpu.di+*(int8*)ip++)); break;
                case 6: return seg_bp+((uint16)(cpu.bp+*(int8*)ip++)); break;
                case 7: return seg+((uint16)(cpu.bx+*(int8*)ip++)); break;
            }
        }
        break;
        case 2:
        if (a32){ 
            switch(i&7){
                case 0: return seg+((uint16)(cpu.eax+*(uint32*)((ip+=4)-4))); break;
                case 1: return seg+((uint16)(cpu.ecx+*(uint32*)((ip+=4)-4))); break;
                case 2: return seg+((uint16)(cpu.edx+*(uint32*)((ip+=4)-4))); break;
                case 3: return seg+((uint16)(cpu.ebx+*(uint32*)((ip+=4)-4))); break;
                case 4: i=sib(); return seg+((uint16)(i+*(uint32*)((ip+=4)-4))); break;
                case 5: return seg_bp+((uint16)(cpu.ebp+*(uint32*)((ip+=4)-4))); break;
                case 6: return seg+((uint16)(cpu.esi+*(uint32*)((ip+=4)-4))); break;
                case 7: return seg+((uint16)(cpu.edi+*(uint32*)((ip+=4)-4))); break;
            }
            }else{
            switch(i&7){
                case 0: return seg+((uint16)(cpu.bx+cpu.si+*(uint16*)((ip+=2)-2))); break;
                case 1: return seg+((uint16)(cpu.bx+cpu.di+*(uint16*)((ip+=2)-2))); break;
                case 2: return seg_bp+((uint16)(cpu.bp+cpu.si+*(uint16*)((ip+=2)-2))); break;
                case 3: return seg_bp+((uint16)(cpu.bp+cpu.di+*(uint16*)((ip+=2)-2))); break;
                case 4: return seg+((uint16)(cpu.si+*(uint16*)((ip+=2)-2))); break;
                case 5: return seg+((uint16)(cpu.di+*(uint16*)((ip+=2)-2))); break;
                case 6: return seg_bp+((uint16)(cpu.bp+*(uint16*)((ip+=2)-2))); break;
                case 7: return seg+((uint16)(cpu.bx+*(uint16*)((ip+=2)-2))); break;
            }
        }
        break;
    }
}

uint16 *rm16(){
    static int32 i;//r/m byte
    i=*ip;
    switch(i>>6){
        case 3:
        ip++; 
        return reg16[i&7];
        break;
        case 0:
        ip++;
        if (a32){
            switch(i&7){
                case 0: return (uint16*)(seg+cpu.ax); break;
                case 1: return (uint16*)(seg+cpu.cx); break;
                case 2: return (uint16*)(seg+cpu.dx); break;
                case 3: return (uint16*)(seg+cpu.bx); break;   
                case 4: return (uint16*)(seg+(uint16)sib_mod0()); break;
                case 5: return (uint16*)(seg+(*(uint16*)((ip+=4)-4))); break;
                case 6: return (uint16*)(seg+cpu.si); break;
                case 7: return (uint16*)(seg+cpu.di); break;
            }
            }else{
            switch(i&7){
                case 0: return (uint16*)(seg+((uint16)(cpu.bx+cpu.si))); break;
                case 1: return (uint16*)(seg+((uint16)(cpu.bx+cpu.di))); break;
                case 2: return (uint16*)(seg_bp+((uint16)(cpu.bp+cpu.si))); break;
                case 3: return (uint16*)(seg_bp+((uint16)(cpu.bp+cpu.di))); break;
                case 4: return (uint16*)(seg+cpu.si); break;
                case 5: return (uint16*)(seg+cpu.di); break;
                case 6: return (uint16*)(seg+(*(uint16*)((ip+=2)-2))); break;
                case 7: return (uint16*)(seg+cpu.bx); break;
            }
        }
        break;
        case 1:
        ip++;
        if (a32){ 
            switch(i&7){
                case 0: return (uint16*)(seg+((uint16)(cpu.eax+*(int8*)ip++))); break;
                case 1: return (uint16*)(seg+((uint16)(cpu.ecx+*(int8*)ip++))); break;
                case 2: return (uint16*)(seg+((uint16)(cpu.edx+*(int8*)ip++))); break;
                case 3: return (uint16*)(seg+((uint16)(cpu.ebx+*(int8*)ip++))); break;
                case 4: i=sib(); return (uint16*)(seg+((uint16)(i+*(int8*)ip++))); break;
                case 5: return (uint16*)(seg_bp+((uint16)(cpu.ebp+*(int8*)ip++))); break;
                case 6: return (uint16*)(seg+((uint16)(cpu.esi+*(int8*)ip++))); break;
                case 7: return (uint16*)(seg+((uint16)(cpu.edi+*(int8*)ip++))); break;
            }
            }else{
            switch(i&7){
                case 0: return (uint16*)(seg+((uint16)(cpu.bx+cpu.si+*(int8*)ip++))); break;
                case 1: return (uint16*)(seg+((uint16)(cpu.bx+cpu.di+*(int8*)ip++))); break;
                case 2: return (uint16*)(seg_bp+((uint16)(cpu.bp+cpu.si+*(int8*)ip++))); break;
                case 3: return (uint16*)(seg_bp+((uint16)(cpu.bp+cpu.di+*(int8*)ip++))); break;
                case 4: return (uint16*)(seg+((uint16)(cpu.si+*(int8*)ip++))); break;
                case 5: return (uint16*)(seg+((uint16)(cpu.di+*(int8*)ip++))); break;
                case 6: return (uint16*)(seg_bp+((uint16)(cpu.bp+*(int8*)ip++))); break;
                case 7: return (uint16*)(seg+((uint16)(cpu.bx+*(int8*)ip++))); break;
            }
        }
        break;
        case 2:
        ip++;
        if (a32){ 
            switch(i&7){
                case 0: return (uint16*)(seg+((uint16)(cpu.eax+*(uint32*)((ip+=4)-4)))); break;
                case 1: return (uint16*)(seg+((uint16)(cpu.ecx+*(uint32*)((ip+=4)-4)))); break;
                case 2: return (uint16*)(seg+((uint16)(cpu.edx+*(uint32*)((ip+=4)-4)))); break;
                case 3: return (uint16*)(seg+((uint16)(cpu.ebx+*(uint32*)((ip+=4)-4)))); break;
                case 4: i=sib(); return (uint16*)(seg+((uint16)(i+*(uint32*)((ip+=4)-4)))); break;
                case 5: return (uint16*)(seg_bp+((uint16)(cpu.ebp+*(uint32*)((ip+=4)-4)))); break;
                case 6: return (uint16*)(seg+((uint16)(cpu.esi+*(uint32*)((ip+=4)-4)))); break;
                case 7: return (uint16*)(seg+((uint16)(cpu.edi+*(uint32*)((ip+=4)-4)))); break;
            }
            }else{
            switch(i&7){
                case 0: return (uint16*)(seg+((uint16)(cpu.bx+cpu.si+*(uint16*)((ip+=2)-2)))); break;
                case 1: return (uint16*)(seg+((uint16)(cpu.bx+cpu.di+*(uint16*)((ip+=2)-2)))); break;
                case 2: return (uint16*)(seg_bp+((uint16)(cpu.bp+cpu.si+*(uint16*)((ip+=2)-2)))); break;
                case 3: return (uint16*)(seg_bp+((uint16)(cpu.bp+cpu.di+*(uint16*)((ip+=2)-2)))); break;
                case 4: return (uint16*)(seg+((uint16)(cpu.si+*(uint16*)((ip+=2)-2)))); break;
                case 5: return (uint16*)(seg+((uint16)(cpu.di+*(uint16*)((ip+=2)-2)))); break;
                case 6: return (uint16*)(seg_bp+((uint16)(cpu.bp+*(uint16*)((ip+=2)-2)))); break;
                case 7: return (uint16*)(seg+((uint16)(cpu.bx+*(uint16*)((ip+=2)-2)))); break;
            }
        }
        break;
    }
}

uint32 *rm32(){
    static int32 i;//r/m byte
    i=*ip;
    switch(i>>6){
        case 3:
        ip++; 
        return reg32[i&7];
        break;
        case 0:
        ip++;
        if (a32){
            switch(i&7){
                case 0: return (uint32*)(seg+cpu.ax); break;
                case 1: return (uint32*)(seg+cpu.cx); break;
                case 2: return (uint32*)(seg+cpu.dx); break;
                case 3: return (uint32*)(seg+cpu.bx); break;
                case 4: return (uint32*)(seg+(uint16)sib_mod0()); break;
                case 5: return (uint32*)(seg+(*(uint16*)((ip+=4)-4))); break;
                case 6: return (uint32*)(seg+cpu.si); break;
                case 7: return (uint32*)(seg+cpu.di); break;
            }
            }else{
            switch(i&7){
                case 0: return (uint32*)(seg+((uint16)(cpu.bx+cpu.si))); break;
                case 1: return (uint32*)(seg+((uint16)(cpu.bx+cpu.di))); break;
                case 2: return (uint32*)(seg_bp+((uint16)(cpu.bp+cpu.si))); break;
                case 3: return (uint32*)(seg_bp+((uint16)(cpu.bp+cpu.di))); break;
                case 4: return (uint32*)(seg+cpu.si); break;
                case 5: return (uint32*)(seg+cpu.di); break;
                case 6: return (uint32*)(seg+(*(uint16*)((ip+=2)-2))); break;
                case 7: return (uint32*)(seg+cpu.bx); break;
            }
        }
        break;
        case 1:
        ip++;
        if (a32){ 
            switch(i&7){
                case 0: return (uint32*)(seg+((uint16)(cpu.eax+*(int8*)ip++))); break;
                case 1: return (uint32*)(seg+((uint16)(cpu.ecx+*(int8*)ip++))); break;
                case 2: return (uint32*)(seg+((uint16)(cpu.edx+*(int8*)ip++))); break;
                case 3: return (uint32*)(seg+((uint16)(cpu.ebx+*(int8*)ip++))); break;
                case 4: i=sib(); return (uint32*)(seg+((uint16)(i+*(int8*)ip++))); break;
                case 5: return (uint32*)(seg_bp+((uint16)(cpu.ebp+*(int8*)ip++))); break;
                case 6: return (uint32*)(seg+((uint16)(cpu.esi+*(int8*)ip++))); break;
                case 7: return (uint32*)(seg+((uint16)(cpu.edi+*(int8*)ip++))); break;
            }
            }else{
            switch(i&7){
                case 0: return (uint32*)(seg+((uint16)(cpu.bx+cpu.si+*(int8*)ip++))); break;
                case 1: return (uint32*)(seg+((uint16)(cpu.bx+cpu.di+*(int8*)ip++))); break;
                case 2: return (uint32*)(seg_bp+((uint16)(cpu.bp+cpu.si+*(int8*)ip++))); break;
                case 3: return (uint32*)(seg_bp+((uint16)(cpu.bp+cpu.di+*(int8*)ip++))); break;
                case 4: return (uint32*)(seg+((uint16)(cpu.si+*(int8*)ip++))); break;
                case 5: return (uint32*)(seg+((uint16)(cpu.di+*(int8*)ip++))); break;
                case 6: return (uint32*)(seg_bp+((uint16)(cpu.bp+*(int8*)ip++))); break;
                case 7: return (uint32*)(seg+((uint16)(cpu.bx+*(int8*)ip++))); break;
            }
        }
        break;
        case 2:
        ip++;
        if (a32){ 
            switch(i&7){
                case 0: return (uint32*)(seg+((uint16)(cpu.eax+*(uint32*)((ip+=4)-4)))); break;
                case 1: return (uint32*)(seg+((uint16)(cpu.ecx+*(uint32*)((ip+=4)-4)))); break;
                case 2: return (uint32*)(seg+((uint16)(cpu.edx+*(uint32*)((ip+=4)-4)))); break;
                case 3: return (uint32*)(seg+((uint16)(cpu.ebx+*(uint32*)((ip+=4)-4)))); break;
                case 4: i=sib(); return (uint32*)(seg+((uint16)(i+*(uint32*)((ip+=4)-4)))); break;
                case 5: return (uint32*)(seg_bp+((uint16)(cpu.ebp+*(uint32*)((ip+=4)-4)))); break;
                case 6: return (uint32*)(seg+((uint16)(cpu.esi+*(uint32*)((ip+=4)-4)))); break;
                case 7: return (uint32*)(seg+((uint16)(cpu.edi+*(uint32*)((ip+=4)-4)))); break;
            }
            }else{
            switch(i&7){
                case 0: return (uint32*)(seg+((uint16)(cpu.bx+cpu.si+*(uint16*)((ip+=2)-2)))); break;
                case 1: return (uint32*)(seg+((uint16)(cpu.bx+cpu.di+*(uint16*)((ip+=2)-2)))); break;
                case 2: return (uint32*)(seg_bp+((uint16)(cpu.bp+cpu.si+*(uint16*)((ip+=2)-2)))); break;
                case 3: return (uint32*)(seg_bp+((uint16)(cpu.bp+cpu.di+*(uint16*)((ip+=2)-2)))); break;
                case 4: return (uint32*)(seg+((uint16)(cpu.si+*(uint16*)((ip+=2)-2)))); break;
                case 5: return (uint32*)(seg+((uint16)(cpu.di+*(uint16*)((ip+=2)-2)))); break;
                case 6: return (uint32*)(seg_bp+((uint16)(cpu.bp+*(uint16*)((ip+=2)-2)))); break;
                case 7: return (uint32*)(seg+((uint16)(cpu.bx+*(uint16*)((ip+=2)-2)))); break;
            }
        }
        break;
    }
}

uint8* seg_es_ptr;
uint8* seg_cs_ptr;
uint8* seg_ss_ptr;
uint8* seg_ds_ptr;
uint8* seg_fs_ptr;
uint8* seg_gs_ptr;

#define seg_es 0
#define seg_cs 1
#define seg_ss 2
#define seg_ds 3
#define seg_fs 4
#define seg_gs 5


#define op_r i&7
void cpu_call(){
    
    static int32 i,i2,i3,x,x2,x3,y,y2,y3;
    static uint8 b,b2,b3;
    static uint8 *uint8p;
    static uint16 *uint16p;
    static uint32 *uint32p;
    static uint8* dseg;
    static int32 r;
    ip=(uint8*)&cmem[cpu.cs*16+cpu.ip];
    
    seg_es_ptr=(uint8*)cmem+cpu.es*16;
    seg_cs_ptr=(uint8*)cmem+cpu.cs*16;
    seg_ss_ptr=(uint8*)cmem+cpu.ss*16;
    seg_ds_ptr=(uint8*)cmem+cpu.ds*16;
    seg_fs_ptr=(uint8*)cmem+cpu.fs*16;
    seg_gs_ptr=(uint8*)cmem+cpu.gs*16;
    
    next_opcode:
    b32=0; a32=0; seg=seg_ds_ptr; seg_bp=seg_ss_ptr;
    
    i=*ip++;
    
    //read any prefixes
    if (i==0x66){b32=1; i=*ip++;}
    if (i==0x26){seg_bp=seg=seg_es_ptr; i=*ip++;}
    if (i==0x2E){seg_bp=seg=seg_cs_ptr; i=*ip++;}
    if (i==0x36){seg=seg_ss_ptr; i=*ip++;}
    if (i==0x3E){seg_bp=seg_ds_ptr; i=*ip++;}
    if (i==0x64){seg_bp=seg=seg_fs_ptr; i=*ip++;}
    if (i==0x65){seg_bp=seg=seg_gs_ptr; i=*ip++;}
    if (i==0x67){a32=1; i=*ip++;}
    
    if (i==0x0F) goto opcode_0F;
    
    r=*ip>>3&7;
    
    //mov
    if (i!=0x8D){
        if (i>=0x88&&i<=0x8E){
            switch(i){
                case 0x88:// /r r/m8,r8
                *rm8()=*reg8[r];
                break;
                case 0x89:// /r r/m16(32),r16(32)
                if (b32) *rm32()=*reg32[r]; else *rm16()=*reg16[r];
                break;
                case 0x8A:// /r r8,r/m8
                *reg8[r]=*rm8();
                break;
                case 0x8B:// /r r16(32),r/m16(32)
                if (b32) *reg32[r]=*rm32(); else *reg16[r]=*rm16();
                break;
                case 0x8C:// /r r/m16,Sreg
                *rm16()=*segreg[r];
                break;
                case 0x8E:// /r Sreg,r/m16
                *segreg[r]=*rm16();
                if (r==0) seg_es_ptr=(uint8*)cmem+*segreg[r]*16;
                //CS (r==1) cannot be set
                if (r==2) seg_ss_ptr=(uint8*)cmem+*segreg[r]*16;
                if (r==3) seg_ds_ptr=(uint8*)cmem+*segreg[r]*16;
                if (r==4) seg_fs_ptr=(uint8*)cmem+*segreg[r]*16;
                if (r==5) seg_gs_ptr=(uint8*)cmem+*segreg[r]*16;
                break;
            }
            goto done;
        }
    }
    if (i>=0xA0&&i<=0xA3){
        switch(i){
            case 0xA0:// al,moffs8
            cpu.al=*(seg+*(uint16*)ip); ip+=2;
            break;
            case 0xA1:// (e)ax,moffs16(32)
            if (b32){cpu.eax=*(uint32*)(seg+*(uint16*)ip); ip+=2;}else{cpu.ax=*(uint16*)(seg+*(uint16*)ip); ip+=2;}
            break;
            case 0xA2:// moffs8,al
            *(seg+*(uint16*)ip)=cpu.al; ip+=2;
            break;
            case 0xA3:// moffs16(32),(e)ax
            if (b32){*(uint32*)(seg+*(uint16*)ip)=cpu.eax; ip+=2;}else{*(uint16*)(seg+*(uint16*)ip)=cpu.ax; ip+=2;}
            break;
        }
        goto done;
    }
    if (i>=0xB0&&i<=0xB7){// +rb reg8,imm8
        *reg8[op_r]=*ip++;
        goto done;
    }
    if (i>=0xB8&&i<=0xBF){// +rw(rd) reg16(32),imm16(32)
        if (b32){*reg32[op_r]=*(uint32*)ip; ip+=4;}else{*reg16[op_r]=*(uint16*)ip; ip+=2;}
        goto done;
    }
    if (i==0xC6){// r/m8,imm8
        uint8p=rm8(); *uint8p=*ip++;
        goto done;
    }
    if (i==0xC7){// r/m16(32),imm16(32)
        if (b32){uint32p=rm32(); *uint32p=*(uint32*)ip; ip+=4;}else{uint16p=rm16(); *uint16p=*(uint16*)ip; ip+=2;}
        goto done;
    }
    
    //ret (todo)
    if (i==0xCB){//(far)
        //assume return control (revise later)
        return;
    }
    if (i==0xCA){//imm16 (far)
        //assume return control (revise later)
        return;
    }
    
    //int (todo)
    if (i==0xCD){
        call_int(*ip++);//assume interrupt table is 0xFFFF
        goto done;
    }
    
    //push
    if (i==0xFF){
        if (b32){*((uint32*)(seg_ss_ptr+(cpu.sp-=4)))=*rm32();}else{*((uint16*)(seg_ss_ptr+(cpu.sp-=2)))=*rm16();}
        goto done;
    }
    if (i>=0x50&&i<=0x57){//+ /r r16(32)
        if (b32){*((uint32*)(seg_ss_ptr+(cpu.sp-=4)))=*reg32[op_r];}else{*((uint16*)(seg_ss_ptr+(cpu.sp-=2)))=*reg16[op_r];}
        goto done;
    }
    if (i==0x6A){//imm8 (sign extended to 16 bits)
        *((uint16*)(seg_ss_ptr+(cpu.sp-=2)))=((int8)*ip++);
        goto done;
    }
    if (i==0x68){//imm16(32)
        if (b32){*((uint32*)(seg_ss_ptr+(cpu.sp-=4)))=*(uint32*)ip; ip+=4;}else{*((uint16*)(seg_ss_ptr+(cpu.sp-=2)))=*(uint16*)ip; ip+=2;}
        goto done;
    }
    if (i==0x0E){//CS
        *((uint16*)(seg_ss_ptr+(cpu.sp-=2)))=*segreg[seg_cs];
        goto done;
    }
    if (i==0x16){//SS
        *((uint16*)(seg_ss_ptr+(cpu.sp-=2)))=*segreg[seg_ss];
        goto done;
    }
    if (i==0x1E){//DS
        *((uint16*)(seg_ss_ptr+(cpu.sp-=2)))=*segreg[seg_ds];
        goto done;
    }
    if (i==0x06){//ES
        *((uint16*)(seg_ss_ptr+(cpu.sp-=2)))=*segreg[seg_es];
        goto done;
    }
    
    //pop
    if (i==0x8F){
        if (b32){*rm32()=*((uint32*)(seg_ss_ptr-4+(cpu.sp+=4)));}else{*rm16()=*((uint16*)(seg_ss_ptr-2+(cpu.sp+=2)));}
        goto done;
    }
    if (i>=0x58&&i<=0x5F){//+rw(d) r16(32)
        if (b32){*reg32[op_r]=*((uint32*)(seg_ss_ptr-4+(cpu.sp+=4)));}else{*reg16[op_r]=*((uint16*)(seg_ss_ptr-2+(cpu.sp+=2)));}
        goto done;
    }
    if (i==0x1F){//DS
        *segreg[seg_ds]=*((uint16*)(seg_ss_ptr-2+(cpu.sp+=2)));
        goto done;
    }
    if (i==0x07){//ES
        
        *segreg[seg_es]=*((uint16*)(seg_ss_ptr-2+(cpu.sp+=2)));
        goto done;
    }
    if (i==0x17){//SS
        *segreg[seg_ss]=*((uint16*)(seg_ss_ptr-2+(cpu.sp+=2)));
        goto done;
    }
    
    goto skip_0F_opcodes;
    opcode_0F:
    i=*ip++;
    r=*ip>>3&7; //required???
    
    //push
    if (i==0xA0){
        *((uint16*)(seg_ss_ptr+(cpu.sp-=2)))=*segreg[seg_fs];
        goto done;
    }
    if (i==0xA8){
        *((uint16*)(seg_ss_ptr+(cpu.sp-=2)))=*segreg[seg_gs];
        goto done;
    }
    
    //pop
    if (i==0xA1){//FS
        *segreg[seg_fs]=*((uint16*)(seg_ss_ptr-2+(cpu.sp+=2)));
        goto done;
    }
    if (i==0xA9){//GS
        *segreg[seg_gs]=*((uint16*)(seg_ss_ptr-2+(cpu.sp+=2)));
        goto done;
    }
    
    skip_0F_opcodes:
    
    i2=((i>>4)&15); if (i2<=9) i2+=48; else i2=i2-10+65;
    unknown_opcode_mess->chr[16]=i2;
    i2=i&15; if (i2<=9) i2+=48; else i2=i2-10+65;
    unknown_opcode_mess->chr[17]=i2;
    MessageBox2(NULL,(char*)unknown_opcode_mess->chr,"X86 Error",MB_OK|MB_SYSTEMMODAL);
    exit(86);
    done:
    if (*ip) goto next_opcode;
    
    exit(cmem[0]);
    
}



int32 screen_last_valid=0;
static std::vector<uint8> screen_last_storage(1,0);
uint8 *screen_last=screen_last_storage.data();
uint32 screen_last_size=static_cast<uint32>(screen_last_storage.size());


uint64 asciicode_value=0;
int32 asciicode_reading=0;



int32 lock_display=0;
int32 lock_display_required=0;

//cost delay, made obselete by managing thread priorities (consider removal)
#define cost_limit 10000
#define cost_delay 0
uint32 cost=0;

// MBF <-> IEEE conversion lives in the Rust runtime (crates/qb-runtime/src/mbf.rs).



int64 build_int64(uint32 val2, uint32 val1){
    int64 val = static_cast<int64>(val2);
    val <<= 32;
    val |= val1;
    return val;
}

uint64 build_uint64(uint32 val2, uint32 val1){
    uint64 val = static_cast<uint64>(val2);
    val <<= 32;
    val |= val1;
    return val;
}

//nb. abreviations are used in variable names to save typing, here are some of the expansions
//cmem=conventional memory
//qbs=qbick basic string (refers to the emulation of quick basic strings)
//sp=stack pointer
//dblock=a 64K memory block in conventional memory holding single variables and strings
extern uint8 *cmem_static_pointer;
uint8 *cmem_static_base=&cmem[0]+1280+65536;
extern uint8 *cmem_dynamic_base;
//[1280][DBLOCK][STATIC-><-DYNAMIC][A000-]

//qbs_cmem_descriptor_space (start of the DBLOCK string area) is defined by the Rust string engine

extern uint32 qbs_cmem_sp; //=256;
extern uint32 cmem_sp; //=65536;
extern ptrszint dblock; //32bit offset of dblock
extern uint64 *nothingvalue;

uint32 qbnex_firsttimervalue;//based on time of day
uint32 clock_firsttimervalue;//based on program launch time




uint8 wait_needed=1;

int32 full_screen=0;//0,1(stretched/closest),2(1:1)
int32 full_screen_set=-1;//0(windowed),1(stretched/closest),2(1:1)


int32 vertical_retrace_in_progress=0;
int32 vertical_retrace_happened=0;





static const char *arrow[] = {
    /* width height num_colors chars_per_pixel */
    "    32    32        3            1",
    /* colors */
    "X c #000000",
    ". c #ffffff",
    "  c None",
    /* pixels */
    "X                               ",
    "XX                              ",
    "X.X                             ",
    "X..X                            ",
    "X...X                           ",
    "X....X                          ",
    "X.....X                         ",
    "X......X                        ",
    "X.......X                       ",
    "X........X                      ",
    "X.........X                     ",
    "X......XXXXX                    ",
    "X...X..X                        ",
    "X..XX..X                        ",
    "X.X  X..X                       ",
    "XX   X..X                       ",
    "X     X..X                      ",
    "      X..X                      ",
    "       X..X                     ",
    "       X..X                     ",
    "        XX                      ",
    "                                ",
    "                                ",
    "                                ",
    "                                ",
    "                                ",
    "                                ",
    "                                ",
    "                                ",
    "                                ",
    "                                ",
    "                                ",
    "0,0"
};


uint8 lock_subsystem=0;

extern uint8 close_program; //=0;
uint8 program_wait=0;

extern uint8 suspend_program;
extern uint8 stop_program;


int32 global_counter=0;
extern double last_line;
void end(void);
extern uint32 new_error;
extern uint32 error_err; //=0;
extern double error_erl; //=0;
extern uint32 error_occurred;
extern uint32 error_goto_line;
extern uint32 error_handling;
extern uint32 error_retry;

void sub__echo(qbs *message);

void unlockvWatchHandle() {
    if (vwatch>0) vwatch=-1;
}

int32 vWatchHandle() {
    return vwatch;
}

void sub__assert(int32 expression, qbs *assert_message, int32 passed) {
    if (asserts==0) return;
    if (expression==0) {
        if (console==1 && passed==1) {
            sub__echo(assert_message);
            error(315);
            return;
        }
        error(314);
    }
    return;
}

//Error machinery: messages, error(), fix_error()'s decisions and the error
//globals live in Rust (crates/qb-runtime/src/error.rs).
char *human_error(int32 errorcode){return (char*)qb_human_error(errorcode);}

qbs *func_command(int32 index, int32 passed);
void QBMAIN(void *);

void fix_error(){
    qbs *c=func_command(0,1);
    if (qb_fix_error(c->chr,c->len)) QBMAIN(NULL);//enter the ON ERROR handler
}

extern "C" int32 qb_host_message_box(const char *message,const char *title,int32 type){
    return MessageBox2(NULL,(char*)message,(char*)title,type);
}
extern "C" void qb_host_end(){end();}

void error(int32 error_number){qb_error(error_number);}

double get_error_erl(){
    return error_erl;
}

uint32 get_error_err(){
    return error_err;
}

void end(){
    qb_fb_capture(display_page,1);//headless framebuffer capture (QBNEX_FB_CAPTURE)
    dont_call_sub_gl=1;
    exit_ok|=1;
    while(!stop_program) Sleep(16);
    while(1) Sleep(16);
}

int32 stop_program_state() {
    return stop_program;
}


//MEM_STATIC memory manager
/*
    mem_static uses a pointer called mem_static_pointer to allocate linear memory.
    It can also change mem_static_pointer back to a previous location, effectively erasing
    any memory after that point.
    Because memory cannot be guaranteed to be allocated in exactly the same location
    after realloc which QBNex requires to keep functionality of previous pointers when
    the current block of memory is full QBNex creates an entirely new block, much larger
    than the previous block (at least 2x), and "writes-off" the previous block as un-
    reclaimable memory. This tradeoff is worth the speed it recovers.
    This allocation strategy can be shown as follows: (X=1MB)
    X
    XX
    XXXX
    XXXXXXXX
    XXXXXXXXXXXXXXXX
    etc.
*/
//mem_static allocator: crates/qb-runtime/src/alloc.rs (globals mem_static,
//mem_static_pointer, mem_static_limit are defined there).
extern uint8 *mem_static;
extern uint8 *mem_static_pointer;
extern uint8 *mem_static_limit;
uint8 *mem_static_malloc(uint32 size){
    int32 e=0; uint8 *p=qb_mem_static_malloc(size,&e);
    if (e) error(e);
    return p;
}
void mem_static_restore(uint8* restore_point){qb_mem_static_restore(restore_point);}

//CMEM_FAR_DYNAMIC memory manager
/*
    (uses a custom "links" based memory manager)
*/
//           &HA000    DBLOCK SIZE        DBLOCK OFFSET
//           655360 - (65536            + 1280         )=588544 links possible
//links limited to 588544/4=147136 (do not have enough links if avg. block size less than 4 bytes)
//stores blocks, not free memory, because blocks are easier to identify
//always scanned from beginning to end, so prev. pointer is unnecessary
//cmem_dynamic allocator: crates/qb-runtime/src/alloc.rs
uint8 *cmem_dynamic_malloc(uint32 size){
    int32 e=0; uint8 *p=qb_cmem_dynamic_malloc(size,&e);
    if (e) error(e);
    return p;
}
void cmem_dynamic_free(uint8 *block){qb_cmem_dynamic_free(block);}

uint8 *defseg=&cmem[1280];//set to base of DBLOCK

void sub_defseg(int32 segment,int32 passed){
    if (new_error) return;
    if (!passed){
        defseg=&cmem[1280];
        return;
    }
    
    if ((segment<-65536)||(segment>65535)){//same range as QB checks
        error(6);
        }else{
        defseg=&cmem[0]+((uint16)segment)*16;
    }
}

int32 func_peek(int32 offset){
    if ((offset<-65536)||(offset>65535)){//same range as QB checks
        error(6);
        return 0;
    }
    return defseg[(uint16)offset];
}

void sub_poke(int32 offset,int32 value){
    if (new_error) return;
    if ((offset<-65536)||(offset>65535)){//same range as QB checks
        error(6);
        return;
    }
    defseg[(uint16)offset]=value;
}

int32 array_ok=1;//kept to compile legacy versions

//gosub-return handling: the stack (next_return_point, return_point, return_points)
//lives in Rust (crates/qb-runtime/src/program.rs).
void more_return_points(){
    int32 e=qb_more_return_points(); if (e) error(e);
}





uint8 keyon[65536];

qbs* singlespace;


//String engine: descriptor pool, tracking lists, qbs_data arena, DBLOCK strings and
//the string functions below live in the Rust runtime (crates/qb-runtime/src/qbs.rs).
//These shims keep the C++ signatures used by qbx.cpp and generated code.
extern uint32 qbs_tmp_list_nexti;//defined by the Rust string engine
void field_free(qbs* str);
extern "C" void qb_host_error(int32 n){error(n);}
extern "C" void qb_host_field_free(qbs *s){field_free(s);}
qbs *qbs_new_descriptor(){return qb_qbs_new_descriptor();}
void qbs_free_descriptor(qbs *str){qb_qbs_free_descriptor(str);}
void qbs_free(qbs *str){qb_qbs_free(str);}
void qbs_cmem_concat_list(){qb_qbs_cmem_concat_list();}
void qbs_concat_list(){qb_qbs_concat_list();}
void qbs_tmp_concat_list(){qb_qbs_tmp_concat_list();}
void qbs_concat(uint32 bytesrequired){qb_qbs_concat(bytesrequired);}
void qbs_concat_cmem(uint32 bytesrequired){qb_qbs_concat_cmem(bytesrequired);}
qbs *qbs_new_cmem(int32 size,uint8 tmp){return qb_qbs_new_cmem(size,tmp);}
qbs *qbs_new_txt(const char *txt){return qb_qbs_new_txt(txt);}
qbs *qbs_new_txt_len(const char *txt,int32 len){return qb_qbs_new_txt_len(txt,len);}
qbs *qbs_new_fixed(uint8 *offset,uint32 size,uint8 tmp){return qb_qbs_new_fixed(offset,size,tmp);}
qbs *qbs_new(int32 size,uint8 tmp){return qb_qbs_new(size,tmp);}
void qbs_maketmp(qbs *str){qb_qbs_maketmp(str);}
qbs *qbs_set(qbs *deststr,qbs *srcstr){return qb_qbs_set(deststr,srcstr);}
qbs *qbs_add(qbs *str1,qbs *str2){return qb_qbs_add(str1,str2);}
qbs *qbs_ucase(qbs *str){return qb_qbs_ucase(str);}
qbs *qbs_lcase(qbs *str){return qb_qbs_lcase(str);}
qbs *func_chr(int32 value){return qb_func_chr(value);}
qbs *func_varptr_helper(uint8 type,uint16 offset){return qb_func_varptr_helper(type,offset);}
qbs *qbs_left(qbs *str,int32 l){return qb_qbs_left(str,l);}
qbs *qbs_right(qbs *str,int32 l){return qb_qbs_right(str,l);}

qbs *func_mksmbf(float val){
    static qbs *tqbs;
    tqbs=qbs_new(4,1);
    if (_fieeetomsbin(&val,(float*)tqbs->chr)==1) {error(5); tqbs->len=0;}
    return tqbs;
}
qbs *func_mkdmbf(double val){
    static qbs *tqbs;
    tqbs=qbs_new(8,1);
    if (_dieeetomsbin(&val,(double*)tqbs->chr)==1) {error(5); tqbs->len=0;}
    return tqbs;
}

float func_cvsmbf(qbs *str){
    static float val;
    if (str->len<4) {error(5); return 0;}
    if (_fmsbintoieee((float*)str->chr,&val)==1) {error(5); return 0;}
    return val;
}
double func_cvdmbf(qbs *str){
    static double val;
    if (str->len<8) {error(5); return 0;}
    if (_dmsbintoieee((double*)str->chr,&val)==1) {error(5); return 0;}
    return val;
}

qbs *b2string(char v){ static qbs *tqbs; tqbs=qbs_new(1,1); *((char*)(tqbs->chr))=v; return tqbs;}
qbs *ub2string(char v){ static qbs *tqbs; tqbs=qbs_new(1,1); *((uint8*)(tqbs->chr))=v; return tqbs;}
qbs *i2string(int16 v){ static qbs *tqbs; tqbs=qbs_new(2,1); *((int16*)(tqbs->chr))=v; return tqbs;}
qbs *ui2string(int16 v){ static qbs *tqbs; tqbs=qbs_new(2,1); *((uint16*)(tqbs->chr))=v; return tqbs;}
qbs *l2string(int32 v){ static qbs *tqbs; tqbs=qbs_new(4,1); *((int32*)(tqbs->chr))=v; return tqbs;}
qbs *ul2string(uint32 v){ static qbs *tqbs; tqbs=qbs_new(4,1); *((uint32*)(tqbs->chr))=v; return tqbs;}
qbs *i642string(int64 v){ static qbs *tqbs; tqbs=qbs_new(8,1); *((int64*)(tqbs->chr))=v; return tqbs;}
qbs *ui642string(uint64 v){ static qbs *tqbs; tqbs=qbs_new(8,1); *((uint64*)(tqbs->chr))=v; return tqbs;}
qbs *s2string(float v){ static qbs *tqbs; tqbs=qbs_new(4,1); *((float*)(tqbs->chr))=v; return tqbs;}
qbs *d2string(double v){ static qbs *tqbs; tqbs=qbs_new(8,1); *((double*)(tqbs->chr))=v; return tqbs;}
qbs *f2string(long double v){ static qbs *tqbs; tqbs=qbs_new(32,1); memset(tqbs->chr,0,32); *((long double*)(tqbs->chr))=v; return tqbs;}
qbs *o2string(ptrszint v){ static qbs *tqbs; tqbs=qbs_new(sizeof(ptrszint),1); memset(tqbs->chr,0,sizeof(ptrszint)); *((ptrszint*)(tqbs->chr))=v; return tqbs;}
qbs *uo2string(uptrszint v){ static qbs *tqbs; tqbs=qbs_new(sizeof(uptrszint),1); memset(tqbs->chr,0,sizeof(uptrszint)); *((uptrszint*)(tqbs->chr))=v; return tqbs;}
qbs *bit2string(uint32 bsize,int64 v){
    static qbs* tqbs;
    tqbs=qbs_new(8,1);
    int64 bmask;
    bmask=~(-(((int64)1)<<bsize));
    *((int64*)(tqbs->chr))=v&bmask;
    tqbs->len=(bsize+7)>>3;
    return tqbs;
}
qbs *ubit2string(uint32 bsize,uint64 v){
    static qbs* tqbs;
    int64 bmask;
    tqbs=qbs_new(8,1);
    bmask=~(-(((int64)1)<<bsize));
    *((uint64*)(tqbs->chr))=v&bmask;
    tqbs->len=(bsize+7)>>3;
    return tqbs;
}

char string2b(qbs*str){ if (str->len<1) {error(5); return 0;} else {return *((char*)str->chr);} }
uint8 string2ub(qbs*str){ if (str->len<1) {error(5); return 0;} else {return *((uint8*)str->chr);} }
int16 string2i(qbs*str){ if (str->len<2) {error(5); return 0;} else {return *((int16*)str->chr);} }
uint16 string2ui(qbs*str){ if (str->len<2) {error(5); return 0;} else {return *((uint16*)str->chr);} }
int32 string2l(qbs*str){ if (str->len<4) {error(5); return 0;} else {return *((int32*)str->chr);} }
uint32 string2ul(qbs*str){ if (str->len<4) {error(5); return 0;} else {return *((uint32*)str->chr);} }
int64 string2i64(qbs*str){ if (str->len<8) {error(5); return 0;} else {return *((int64*)str->chr);} }
uint64 string2ui64(qbs*str){ if (str->len<8) {error(5); return 0;} else {return *((uint64*)str->chr);} }
float string2s(qbs*str){ if (str->len<4) {error(5); return 0;} else {return *((float*)str->chr);} }
double string2d(qbs*str){ if (str->len<8) {error(5); return 0;} else {return *((double*)str->chr);} }
long double string2f(qbs*str){ if (str->len<32) {error(5); return 0;} else {return *((long double*)str->chr);} }
ptrszint string2o(qbs*str){ if (str->len<sizeof(ptrszint)) {error(5); return 0;} else {return *((ptrszint*)str->chr);} }
uptrszint string2uo(qbs*str){ if (str->len<sizeof(uptrszint)) {error(5); return 0;} else {return *((uptrszint*)str->chr);} }
uint64 string2ubit(qbs*str,uint32 bsize){
    int64 bmask;
    if (str->len<((bsize+7)>>3)) {error(5); return 0;}
    bmask=~(-(((int64)1)<<bsize));
    return (*(uint64*)str->chr)&bmask;
}
int64 string2bit(qbs*str,uint32 bsize){
    int64 bmask, bval64;
    if (str->len<((bsize+7)>>3)) {error(5); return 0;}
    bmask=~(-(((int64)1)<<bsize));
    bval64=(*(uint64*)str->chr)&bmask;
    if (bval64&(((int64)1)<<(bsize-1))) return (bval64|(~bmask));
    return bval64;
}

void lrset_field(qbs *str);

void sub_lset(qbs *dest,qbs *source){
    if (new_error) return;
    qb_lset(dest->chr,dest->len,source->chr,source->len);//Rust: crates/qb-runtime/src/fileio.rs
    if (dest->field) lrset_field(dest);
}

void sub_rset(qbs *dest,qbs *source){
    if (new_error) return;
    qb_rset(dest->chr,dest->len,source->chr,source->len);//Rust: crates/qb-runtime/src/fileio.rs
    if (dest->field) lrset_field(dest);
}




qbs *func_space(int32 spaces){return qb_func_space(spaces);}
qbs *func_string(int32 characters,int32 asciivalue){return qb_func_string(characters,asciivalue);}
void set_qbs_size(ptrszint *target_qbs,int32 newlength){qb_set_qbs_size(target_qbs,newlength);}
int32 func_instr(int32 start,qbs *str,qbs *substr,int32 passed){return qb_func_instr(start,str,substr,passed);}
int32 func__instrrev(int32 start,qbs *str,qbs *substr,int32 passed){return qb_func__instrrev(start,str,substr,passed);}
void sub_mid(qbs *dest,int32 start,int32 l,qbs* src,int32 passed){
    if (new_error) return;
    if (dest==nothingstring) return;//quiet exit, error has already been reported!
    qb_sub_mid(dest,start,l,src,passed);
}
qbs *func_mid(qbs *str,int32 start,int32 l,int32 passed){return qb_func_mid(str,start,l,passed);}
qbs *qbs_ltrim(qbs *str){return qb_qbs_ltrim(str);}
qbs *qbs_rtrim(qbs *str){return qb_qbs_rtrim(str);}
qbs *qbs__trim(qbs *str){return qb_qbs__trim(str);}
int32 func__str_nc_compare(qbs *s1, qbs *s2){return qb_func__str_nc_compare(s1,s2);}
int32 func__str_compare(qbs *s1, qbs *s2){return qb_func__str_compare(s1,s2);}

qbs *qbs_inkey(){
    if (new_error) return qbs_new(0,1);
    qbs *tqbs;
    // Sleep(0);
    tqbs=qbs_new(2,1);
    if (cmem[0x41a]!=cmem[0x41c]){
        tqbs->chr[0]=cmem[0x400+cmem[0x41a]];
        tqbs->chr[1]=cmem[0x400+cmem[0x41a]+1];
        if (tqbs->chr[0]){
            tqbs->len=1;
            }else{
            if (tqbs->chr[1]==0) tqbs->len=1;
        }
        cmem[0x41a]+=2;
        if (cmem[0x41a]==62) cmem[0x41a]=30;
        }else{
        tqbs->len=0;
    }
    return tqbs;
}

void sub__keyclear(int32 buf, int32 passed) {
    if (new_error) return;
    if (passed && (buf > 3 || buf < 1)) error(5);
    //  Sleep(10);
    if ((buf == 1 && passed) || !passed) {
        //INKEY$ buffer
        cmem[0x41a]=30; cmem[0x41b]=0; //head
        cmem[0x41c]=30; cmem[0x41d]=0; //tail
    }
    if ((buf == 2 && passed) || !passed) {
        //_KEYHIT buffer
        qb_keyhit_clear();
    }
    if ((buf == 3 && passed) || !passed) {
        //INP(&H60) buffer
        port60h_events = 0;
    }
    #ifdef QBNex_WINDOWS
        //Windows Console Buffer
        FlushConsoleInputBuffer(GetStdHandle (STD_INPUT_HANDLE));
    #endif


}

//STR() functions: number formatting lives in the Rust runtime (crates/qb-runtime/src/numfmt.rs).
static qbs *qbs_str_from_rust(const unsigned char *text, int32 len){
    qbs *tqbs=qbs_new(len,1);
    memcpy(tqbs->chr,text,len);
    return tqbs;
}
qbs *qbs_str(int64 value){unsigned char b[QB_STR_MAX]; return qbs_str_from_rust(b,qb_str_i64(value,b));}
qbs *qbs_str(int32 value){return qbs_str((int64)value);}
qbs *qbs_str(int16 value){return qbs_str((int64)value);}
qbs *qbs_str(int8 value){return qbs_str((int64)value);}
qbs *qbs_str(uint64 value){unsigned char b[QB_STR_MAX]; return qbs_str_from_rust(b,qb_str_u64(value,b));}
qbs *qbs_str(uint32 value){return qbs_str((uint64)value);}
qbs *qbs_str(uint16 value){return qbs_str((uint64)value);}
qbs *qbs_str(uint8 value){return qbs_str((uint64)value);}
qbs *qbs_str(float value){unsigned char b[QB_STR_MAX]; return qbs_str_from_rust(b,qb_str_f32(value,b));}
qbs *qbs_str(double value){unsigned char b[QB_STR_MAX]; return qbs_str_from_rust(b,qb_str_f64(value,b));}
qbs *qbs_str(long double value){return qbs_str((double)value);}


int32 qbs_equal(qbs *str1,qbs *str2){return qb_qbs_equal(str1,str2);}
int32 qbs_notequal(qbs *str1,qbs *str2){return qb_qbs_notequal(str1,str2);}
int32 qbs_greaterthan(qbs *str2,qbs *str1){return qb_qbs_greaterthan(str2,str1);}
int32 qbs_lessthan(qbs *str1,qbs *str2){return qb_qbs_lessthan(str1,str2);}
int32 qbs_lessorequal(qbs *str1,qbs *str2){return qb_qbs_lessorequal(str1,str2);}
int32 qbs_greaterorequal(qbs *str2,qbs *str1){return qb_qbs_greaterorequal(str2,str1);}
int32 qbs_asc(qbs *str,uint32 i){return qb_qbs_asc(str,i);}
int32 qbs_asc(qbs *str){return qb_qbs_asc1(str);}
int32 qbs_len(qbs *str){return qb_qbs_len(str);}


//QBG BLOCK
int32 qbg_mode=-1;//-1 means not initialized!
int32 qbg_text_only;
//text & graphics modes
int32 qbg_height_in_characters, qbg_width_in_characters;
int32 qbg_top_row, qbg_bottom_row;
int32 qbg_cursor_x, qbg_cursor_y;
int32 qbg_character_height, qbg_character_width;
uint32 qbg_color, qbg_background_color;
//text mode ONLY
int32 qbg_cursor_show;
int32 qbg_cursor_firstvalue, qbg_cursor_lastvalue;//these values need revision
//graphics modes ONLY
int32 qbg_width, qbg_height;
float qbg_x, qbg_y;
int32 qbg_bits_per_pixel, qbg_pixel_mask; //for monochrome modes 1b, for 16 color 1111b, for 256 color 11111111b
int32 qbg_bytes_per_pixel;
int32 qbg_clipping_or_scaling;//1=clipping, 2=clipping and scaling
int32 qbg_view_x1, qbg_view_y1, qbg_view_x2, qbg_view_y2;
int32 qbg_view_offset_x, qbg_view_offset_y;
float qbg_scaling_x, qbg_scaling_y;
float qbg_scaling_offset_x, qbg_scaling_offset_y;
float qbg_window_x1, qbg_window_y1, qbg_window_x2, qbg_window_y2;
int32 qbg_pages;
uint32 *qbg_pageoffsets;
int32 *qbg_cursor_x_previous; //used to recover old cursor position
int32 *qbg_cursor_y_previous;
int32 qbg_active_page;
uint8 *qbg_active_page_offset;
int32 qbg_visual_page;
uint8 *qbg_visual_page_offset;
int32 qbg_color_assign[256];//for modes with quasi palettes!















void qbg_palette(uint32 attribute,uint32 col,int32 passed){
    if (new_error) return;
    if (int32 e=qb_gfx_palette(write_page,attribute,col,passed)) error(e);
}




void qbg_sub_color(uint32 col1,uint32 col2,uint32 bordercolor,int32 passed){
    if (new_error) return;
    if (!passed){
        //performs no action if nothing passed (as in QBASIC for some modes)
        return;
    }

	if (write_page->console){
		#ifdef QBNex_WINDOWS
			HANDLE output = GetStdHandle(STD_OUTPUT_HANDLE);
			int color = col2 * 16 + col1;
			SetConsoleTextAttribute(output, color);
		#else
            // Exactly how the colour is rendered depends on your terminal emulator and
            // colour palette. Themes and user-customisation aside, the first 16 colours
            // line up with the old VGA colour scheme.
            // Most terminal emulators can handle 8 bit colour, see
            // https://en.wikipedia.org/wiki/ANSI_escape_code#8-bit for the 8 bit colour palette.
            if ((passed & 1 && (col1 > 255 || col1 < 0))
                    || (passed & 2 && (col2 > 255 || col2 < 0)))
                goto error;

            if (passed & 1)
                printf("\033[38;5;%dm", col1);

            if (passed & 2)
                printf("\033[48;5;%dm", col2);
		#endif
        return;
	}
    if (int32 e=qb_gfx_color(write_page,col1,col2,passed)) error(e);
    return;
    #ifndef QBNex_WINDOWS
    error:
    error(5);
    return;
    #endif
}

void defaultcolors(){qb_gfx_defaultcolors(write_page);}

//Note: Cannot be used to setup page 0, just to validate it
void validatepage(int32 n){
    static int32 i,i2;
    //add new page indexes if necessary
    if (n>=pages){
        ensure_page_entries(n+1);
    }
    //create page at index n if none exists
    if (!page[n]){
        //graphical (assumed)
        i=page[0];
        i2=imgnew(img[i].width,img[i].height,img[i].compatible_mode);
        //modify based on page 0's attributes
        //i. link palette to page 0's palette (if necessary)
        if (img[i2].bytes_per_pixel!=4){
            qb_release_img_palette(i2); img[i2].flags^=IMG_FREEPAL;
            qb_set_img_external_palette(i2,img[i].pal);
        }
        //ii. set flags
        img[i2].flags|=IMG_SCREEN;
        //iii. inherit font
        selectfont(img[i].font,&img[i2]);
        //text
        //...
        page[n]=i2;
    }
    return;
}//validate_page



void qbg_screen(int32 mode,int32 color_switch,int32 active_page,int32 visual_page,int32 refresh,int32 passed){
    if (new_error) return;
    
    if (width8050switch){
        if ((passed!=1)||mode) width8050switch=0;
    }
    
    static int32 i,i2,i3,x,y,f,p;
    static img_struct *im;
    static int32 prev_width_in_characters,prev_height_in_characters;
    
    static int32 last_active_page=0;//used for active page settings migration
    
    i=0;//update flags
    //1=mode change required
    //2=page change required (used only to see if an early exit without locking is possible)
    
    i2=page[0];
    if (passed&1){//mode
        if (mode<0){//custom screen
            i3=-mode;
            if (i3>=nextimg){error(258); return;}//within valid range?
            if (!img[i3].valid){error(258); return;}//valid? 
            if (i3!=i2) i=1; //is mode changing?
            }else{
            if (!qb_gfx_screen_mode_valid(mode)) goto error;//modes 3-6 and >13
            //is mode changing?
            if (i2){
                if (img[i2].compatible_mode!=mode) i=1;
            }else i=1;
            //force update if special parameters passed
            //(at present, only SCREEN 0 is ever called with these overrides, so handling
            // of these is done only in the SCREEN 0 section of the SCREEN sub)
            if ((sub_screen_width_in_characters!=-1)||(sub_screen_height_in_characters!=-1)||(sub_screen_font!=-1)) i=1;
        }
    }
    
    if (passed&4){//active page
        if (active_page<0) goto error;
        if (!(passed&8)){//if visual page not specified, set it to the active page
            passed|=8;
            visual_page=active_page;
        }
        if (!(i&1)){//mode not changing
            //validate the passed active page, then see if it is the currently selected page
            validatepage(active_page); i2=page[active_page];
            if ((i2!=read_page_index)||(i2!=write_page_index)) i|=2;
        }
    }//passed&4
    
    if (passed&8){//visual page
        i3=visual_page;
        if (i3<0) goto error;
        if (!(i&1)){//mode not changing
            validatepage(visual_page); i2=page[visual_page];
            if (i2!=display_page_index) i|=2;
        }
    }//passed&8
    
    //if no changes need to be made exit before locking
    if (!i) return;
    
    if (autodisplay){
        if (lock_display_required){//on init of main (), attempting a lock would create an infinite loop
            if (i&1){//avoid locking when only changing the screen page
                if (lock_display==0) lock_display=1;//request lock
                while (lock_display!=2){
                    Sleep(0);
                }
            }
        }
    }
    
    screen_last_valid=0;//ignore cache used to update the screen on next update
    
    if (passed&1){//mode
        if (i&1){//mode change necessary
            
            //calculate previous width & height if possible
            prev_width_in_characters=0; prev_height_in_characters=0; 
            if (i=page[0]){//currently in a screen mode?
                im=&img[i];
                if (!im->compatible_mode){
                    prev_width_in_characters=im->width; prev_height_in_characters=im->height;
                    }else{
                    x=fontwidth[im->font]; if (!x) x=1;
                    prev_width_in_characters=im->width/x;
                    prev_height_in_characters=im->height/fontheight[im->font];
                }
            }//currently in a screen mode
            
            
            //free any previously allocated surfaces
            //free pages in reverse order
            if (page[0]){//currently in a screen mode?
                for (i=1;i<pages;i++){
                    if(i2=page[i]){
                        //manual delete, freeing video pages is usually illegal
                        qb_release_img_owned_resources(i2);
                        freeimg(i2);
                    }//i2
                }//i
                i=page[0];
                if (sub_screen_keep_page0){
                    img[i].flags^=IMG_SCREEN;
                    }else{
                    qb_release_img_owned_resources(i);
                    freeimg(i);
                }
            }//currently in a screen mode
            sub_screen_keep_page0=0;//reset to default status
            
            pages=1; page[0]=0;
            
            if (mode<0){//custom screen
                i=-mode;
                page[0]=i; img[i].flags|=IMG_SCREEN; select_all_pages(i);
                sub_screen_keep_page0=1;
            }
            
            //320 x 200 graphics
            //40 x 25 text format, character box size of 8 x 8
            //Assignment of up to 256K colors to up to 256 attributes
            //graphics modes: size and 8x8 font override from Rust (qb-runtime gfx/screen.rs)
            if (mode>0){
                int32 geo[3];
                if (qb_gfx_screen_geometry(mode,prev_width_in_characters,prev_height_in_characters,geo)){
                    if (mode==13){
                        i=imgframe(&cmem[655360],geo[0],geo[1],13);
                        memset(img[i].offset,0,320*200);
                    }else{
                        i=imgnew(geo[0],geo[1],mode);
                        if (geo[2]) selectfont(geo[2],&img[i]);//override default font
                    }
                    page[0]=i; img[i].flags|=IMG_SCREEN; select_all_pages(i);
                }
            }
            
            //640 x 480 graphics
            //80 x 30 or 80 x 60 text format, character box size of 8 x 16 or 8 x 8
            //Assignment of up to 256K colors to 16 attributes
            if (mode==12){
                i=imgnew(640,480,12);
                if ((prev_width_in_characters==80)&&(prev_height_in_characters==60)) selectfont(8,&img[i]);//override default font
                page[0]=i; img[i].flags|=IMG_SCREEN; select_all_pages(i);
            }//12
            
            /*
                Screen 11
                \A6 640 x 480 graphics
                \A6 80 x 30 or 80 x 60 text format, character box size of 8 x 16 or 8 x 8
                \A6 Assignment of up to 256K colors to 2 attributes
            */
            if (mode==11){
                i=imgnew(640,480,11);
                if ((prev_width_in_characters==80)&&(prev_height_in_characters==60)) selectfont(8,&img[i]);//override default font
                page[0]=i; img[i].flags|=IMG_SCREEN; select_all_pages(i);
            }//11
            
            //SCREEN 10: 640 x 350 graphics, monochrome monitor only
            //  \A6 80 x 25 or 80 x 43 text format, 8 x 14 or 8 x 8 character box size
            //  \A6 128K page size, page range is 0 (128K) or 0-1 (256K)
            //  \A6 Up to 9 pseudocolors assigned to 4 attributes
            /*
                'colors swap every half second!
                'using PALETTE does NOT swap color indexes
                '0 black-black
                '1 black-grey
                '2 black-white
                '3 grey-black
                '4 grey-grey
                '5 grey-white
                '6 white-black
                '7 white-grey
                '8 white-white
                '*IMPORTANT* QB sets initial values up different to default palette!
                '0 block-black(0)
                '1 grey-grey(4)
                '2 white-black(6)
                '3 white-white(8)
            */
            if (mode==10){
                i=imgnew(640,350,10);
                if ((prev_width_in_characters==80)&&(prev_height_in_characters==43)) selectfont(8,&img[i]);//override default font
                page[0]=i; img[i].flags|=IMG_SCREEN; select_all_pages(i);
                
            }//10
            
            /*
                SCREEN 9: 640 x 350 graphics
                \A6 80 x 25 or 80 x 43 text format, 8 x 14 or 8 x 8 character box size
                \A6 64K page size, page range is 0 (64K);
                128K page size, page range is 0 (128K) or 0-1 (256K)
                \A6 16 colors assigned to 4 attributes (64K adapter memory), or
                64 colors assigned to 16 attributes (more than 64K adapter memory)
            */
            if (mode==9){
                i=imgnew(640,350,9);
                if ((prev_width_in_characters==80)&&(prev_height_in_characters==43)) selectfont(8,&img[i]);//override default font
                page[0]=i; img[i].flags|=IMG_SCREEN; select_all_pages(i);
            }//9
            
            /*
                SCREEN 8: 640 x 200 graphics
                \A6 80 x 25 text format, 8 x 8 character box
                \A6 64K page size, page ranges are 0 (64K), 0-1 (128K), or 0-3 (246K)
                \A6 Assignment of 16 colors to any of 16 attributes
            */
            if (mode==8){
                i=imgnew(640,200,8);
                page[0]=i; img[i].flags|=IMG_SCREEN; select_all_pages(i);
            }//8
            
            /*
                SCREEN 7: 320 x 200 graphics
                \A6 40 x 25 text format, character box size 8 x 8
                \A6 32K page size, page ranges are 0-1 (64K), 0-3 (128K), or 0-7 (256K)
                \A6 Assignment of 16 colors to any of 16 attributes
            */
            if (mode==7){
                i=imgnew(320,200,7);
                page[0]=i; img[i].flags|=IMG_SCREEN; select_all_pages(i);
            }//7
            
            /*
                SCREEN 4:
                \A6 Supports Olivetti (R) Personal Computers models M24, M240, M28,
                M280, M380, M380/C, M380/T and AT&T (R) Personal Computers 6300
                series
                \A6 640 x 400 graphics
                \A6 80 x 25 text format, 8 x 16 character box
                \A6 1 of 16 colors assigned as the foreground color (selected by the
                COLOR statement); background is fixed at black.
            */
            //Note: QBNex will not support SCREEN 4
            
            /*
                SCREEN 3: Hercules adapter required, monochrome monitor only
                \A6 720 x 348 graphics
                \A6 80 x 25 text format, 9 x 14 character box
                \A6 2 screen pages (1 only if a second display adapter is installed)
                \A6 PALETTE statement not supported
            */
            //Note: QBNex will not support SCREEN 3
            
            /*
                SCREEN 2: 640 x 200 graphics
                \A6 80 x 25 text format with character box size of 8 x 8
                \A6 16 colors assigned to 2 attributes with EGA or VGA
            */
            if (mode==2){
                i=imgnew(640,200,2);
                page[0]=i; img[i].flags|=IMG_SCREEN; select_all_pages(i);
            }//2
            
            /*
                SCREEN 1: 320 x 200 graphics
                \A6 40 x 25 text format, 8 x 8 character box
                \A6 16 background colors and one of two sets of 3 foreground colors assigned
                using COLOR statement with CGA
                \A6 16 colors assigned to 4 attributes with EGA or VGA
            */
            if (mode==1){
                i=imgnew(320,200,1);
                page[0]=i; img[i].flags|=IMG_SCREEN; select_all_pages(i);
            }//1
            
            /*
                MDPA, CGA, EGA, or VGA Adapter Boards
                SCREEN 0: Text mode only
                \A6 Either 40 x 25, 40 x 43, 40 x 50, 80 x 25, 80 x 43, or 80 x 50 text format
                with 8 x 8 character box size (8 x 14, 9 x 14, or 9 x 16 with EGA or VGA)
                \A6 16 colors assigned to 2 attributes
                \A6 16 colors assigned to any of 16 attributes (with CGA or EGA)
                \A6 64 colors assigned to any of 16 attributes (with EGA or VGA)
            */
            /*
                granularity from &HB800
                4096 in 80x25
                2048 in 40x25
                6880 in 80x43 (80x43x2=6880)
                3440 in 40x43 (40x43x2=3440)
                8000 in 80x50 (80x50x2=8000)
                4000 in 40x50 (40x50x2=4000)
            */
            if (mode==0){
                
                //text layout, font and cmem page granularity: Rust (qb-runtime gfx/screen.rs)
                {int32 lay[5];
                qb_gfx_screen0_layout(prev_width_in_characters,prev_height_in_characters,&sub_screen_width_in_characters,&sub_screen_height_in_characters,&sub_screen_font,lay);
                x=lay[0]; y=lay[1]; f=lay[2]; i2=lay[3]; p=lay[4];}
                //make sure 8 page indexes exist
                if (7>=pages){
                    ensure_page_entries(8);
                }
                for (i3=0;i3<8;i3++){
                    if (i3<p){
                        i=imgframe(&cmem[753664+i2*i3],x,y,0);
                        }else{
                        i=imgnew(x,y,0);
                    }
                    selectfont(f,&img[i]);
                    img[i].flags|=IMG_SCREEN;
                    page[i3]=i;
                }
                //text-clear 64K after seg. &HB800
                for (i=0;i<65536;i+=2){cmem[753664+i]=32; cmem[753664+i+1]=7;}//init. 64K of memory after B800
                i=page[0];
                select_all_pages(i);
            }//0
            
            write_page->draw_ta=0.0; write_page->draw_scale=1.0; //reset DRAW attributes (of write_page)
            
            last_active_page=0;
            
            key_display_redraw=1; key_update();
            
        }//setmode
    }//passed MODE
    
    //note: changing the active or visual page reselects the default colors!
    if (passed&4){//SCREEN ?,?,X,? (active_page)
        i=active_page; validatepage(i); i=page[i];
        if ((write_page_index!=i)||(read_page_index!=i)){
            select_write_and_read_page(i);
            defaultcolors();
            //reset VIEW PRINT state
            write_page->top_row=1;
            if (!write_page->text) write_page->bottom_row=(write_page->height/write_page->font); else write_page->bottom_row=write_page->height;
            if (write_page->cursor_y>write_page->bottom_row) write_page->cursor_y=1;
            write_page->bottom_row--; if (write_page->bottom_row<=0) write_page->bottom_row=1;
            
            //active page migration
            //note: transfers any screen settings which are maintained during a QBASIC active page switch
            if (last_active_page!=active_page){
                static img_struct *old_page;
                old_page=&img[page[last_active_page]];
                //WINDOW settings
                /*
                    SCREEN 7
                    WINDOW (0, 0)-(1, 1)
                    SCREEN 7, , 1, 1
                    PSET (.5, .5), 14
                */
                //VIEW settings
                /*
                    SCREEN 7
                    VIEW SCREEN (50, 50)-(100, 100)
                    SCREEN 7, , 1, 1
                    LINE (0, 0)-(1000, 1000), 1, BF
                */
                //GRAPHICS CURSOR LOCATION
                //(proven)
                //NOT MAINTAINED:
                // X color settings (for both text and graphics)
                // X text cursor location
                // X draw color (reset, as in QBASIC, by defaultcolors())
                if (!write_page->text){
                    memcpy(&write_page->apm_p1,&old_page->apm_p1,(uint32)(&write_page->apm_p2-&write_page->apm_p1));
                }
                last_active_page=active_page;
            }//active page migration
            
        }
        
    }//passed&4
    
    if (passed&8){//SCREEN ?,?,?,X (visual_page)
        i=visual_page; validatepage(i); i=page[i];
        if (display_page_index!=i){
            select_display_page(i);
            defaultcolors();
            //reset VIEW PRINT state
            write_page->top_row=1;
            if (!write_page->text) write_page->bottom_row=(write_page->height/write_page->font); else write_page->bottom_row=write_page->height;
            if (write_page->cursor_y>write_page->bottom_row) write_page->cursor_y=1;
            write_page->bottom_row--; if (write_page->bottom_row<=0) write_page->bottom_row=1;
            
        }
    }//passed&8
    
    if (autodisplay){
        if (lock_display_required) lock_display=0;//release lock
    }
    
    return;
    error:
    error(5);
    return;
}//screen (end)

void sub_pcopy(int32 src,int32 dst){
    if (new_error) return;
    static int32 src_index,dst_index;
    static img_struct *s,*d;
    s=resolve_page_or_image_ptr(src,src_index); if (!s){error(5); return;}
    d=resolve_page_or_image_ptr(dst,dst_index); if (!d){error(5); return;}
    if (int32 e=qb_gfx_pcopy(s,d)) error(e);
}

void qbsub_width(int32 option,int32 value1,int32 value2,int32 value3, int32 value4, int32 passed){
    //[{#|LPRINT}][?],[?]
    static int32 i,i2;
    
    if (new_error) return;
    
    if (option==0){//WIDTH [?][,?]
        
        width8050switch=0;
        
        static uint32 col,col2;
        
        //used to restore scaling after simple font changes
        //QBASIC/4.5/7.1: PMAP uses old scaling values after WIDTH change
        static float window_x1,window_y1,window_x2,window_y2;
        
        //Specifics:
        //MODE 0: Changes the resolution based on the desired width
        //        Horizontal width of 1 to 40 uses a double width font
        //        Heights from 1 to 42 use font height 16 pixels
        //        Heights from 43 to 49 use font height 14 pixels
        //        Heights from 50 to ? use font height 8 pixels
        //MODES 1-13: The resolution IS NOT CHANGED
        //            The font is changed to a font usually available for that screen
        //            mode, if available, that fits the given dimensions EXACTLY
        //            If not possible, it may jump back to SCREEN 0 in some instances
        //            just as it did in QBASIC
        //256/32 BIT MODES: The font is unchanged
        //                  The resolution is changed using the currently selected font
        //note:
        //COLOR selection is kept, all other values are lost (if staying in same "mode")
        static int32 f,f2,width,height;
        
        width=value1; height=value2;

        #ifdef QBNex_WINDOWS
            if (write_page->console){
                SECURITY_ATTRIBUTES SecAttribs = {sizeof(SECURITY_ATTRIBUTES), 0, 1};
                HANDLE cl_conout = CreateFileA("CONOUT$", GENERIC_READ | GENERIC_WRITE, FILE_SHARE_READ | FILE_SHARE_WRITE, & SecAttribs, OPEN_EXISTING, 0, 0);
                HANDLE hConsole = GetStdHandle(STD_OUTPUT_HANDLE);
                CONSOLE_SCREEN_BUFFER_INFO cl_bufinfo;
                
                GetConsoleScreenBufferInfo(cl_conout, &cl_bufinfo); //get the screen buffer information, for blank entries

                if (width<=0) width = cl_bufinfo.srWindow.Right - cl_bufinfo.srWindow.Left + 1;; //if width is omitted, then use existing width
                if (height<=0) height = cl_bufinfo.srWindow.Bottom - cl_bufinfo.srWindow.Top + 1;; //if height is omitted, then use existing height
                if (value3<=0) value3 = cl_bufinfo.dwSize.X; //if bufferwidth is omitted, then use existing buffer width
                if (value4<=0) value4 = cl_bufinfo.dwSize.Y; //same as above, but for height
                if (value3<value1)value3=value1; //don't make the buffer width smaller than the console width itself
                if (value4<value2)value4=value2; //and don't make that buffer height smaller than the console height

                SMALL_RECT rect = {0,0, width-1, height-1};
                COORD bufferSize = {value3, value4};
                SetConsoleScreenBufferSize(hConsole, bufferSize); //set the buffer
                SetConsoleWindowInfo(hConsole, TRUE, &rect); //set the console itself
                return;
            }
        #endif

        if ((!(passed&1))&&(!(passed&2))) goto error;//cannot omit both arguments
        
        if ((write_page->compatible_mode==32)||(write_page->compatible_mode==256)){
            
            if (!(passed&1)){//width ommited
                width=write_page->width;
                }else{
                if (width<=0) goto error;
                i=fontwidth[write_page->font]; if (!i) i=1;
                width*=i;
            }
            
            if (!(passed&2)){//height ommited
                height=write_page->height;
                }else{
                if (height<=0) goto error;
                height*=fontheight[write_page->font];
            }
            
            //width & height are now the desired dimensions
            
            if ((width==write_page->width)&&(height==write_page->height)) return;//no change required
            
            if (write_page->flags&IMG_SCREEN){
                //delete pages 1-?
                for(i=1;i<pages;i++){
                    if(i2=page[i]){
                        if (display_page_index==i2){display_page_index=page[0]; display_page=&img[display_page_index];}
                        if (read_page_index==i2){read_page_index=display_page_index; read_page=display_page;}
                        if (write_page_index==i2){write_page_index=display_page_index; write_page=display_page;}
                        //manual delete, freeing video pages is usually illegal
                        qb_release_img_owned_resources(i2);
                        freeimg(i2);
                    }
                }//i
            }
            
            if (autodisplay){
                if (write_page->flags&IMG_SCREEN){
                    if (lock_display_required){
                        if (lock_display==0) lock_display=1;
                        while (lock_display!=2){
                            Sleep(0);
                        }
                    }
                }
            }
            
            col=write_page->color; col2=write_page->background_color;
            f=write_page->font;
            //change resolution
            write_page->width=width; write_page->height=height;
            if (write_page->flags&IMG_FREEMEM){
                qb_release_img_pixels(write_page_index);
                qb_allocate_img_pixels(write_page_index,width*height*write_page->bytes_per_pixel);
                }else{//frame?
                memset(write_page->offset,0,width*height*write_page->bytes_per_pixel);
            }
            imgrevert(write_page_index);
            write_page->color=col; write_page->background_color=col2;
            selectfont(f,write_page);
            
            if (autodisplay){
                if (write_page->flags&IMG_SCREEN){
                    if (lock_display_required) lock_display=0;//release lock
                }
            }
            
            return;
            
        }//32/256
        
        if (!(passed&1)){//width ommited
            if (height<=0) goto error;
            
            if (!write_page->compatible_mode){//0
                f=8;
                if (height<=49) f=14;
                if (height<=42) f=16;
                width=write_page->width;
                if (width<=40) f++;
                if ((write_page->font==f)&&(write_page->height==height)) return;//no change
                sub_screen_height_in_characters=height; sub_screen_width_in_characters=width;
                sub_screen_font=f;
                qbg_screen(0,0,0,0,0,1);
                return;
            }//0
            
            if (((write_page->compatible_mode>=1)&&(write_page->compatible_mode<=8))||(write_page->compatible_mode==13)){
                if (write_page->height==height*8){//correct resolution
                    if (write_page->font==8) return;//correct font, no change required
                    if (write_page->flags&IMG_SCREEN){
                        //delete pages 1-?
                        for(i=1;i<pages;i++){
                            if(i2=page[i]){
                                if (display_page_index==i2){display_page_index=page[0]; display_page=&img[display_page_index];}
                                if (read_page_index==i2){read_page_index=display_page_index; read_page=display_page;}
                                if (write_page_index==i2){write_page_index=display_page_index; write_page=display_page;}
                                //manual delete, freeing video pages is usually illegal
                                qb_release_img_owned_resources(i2);
                                freeimg(i2);
                            }
                        }//i
                    }
                    col=write_page->color; col2=write_page->background_color;
                    imgrevert(write_page_index);
                    write_page->color=col; write_page->background_color=col2;
                    selectfont(8,write_page);
                    return;
                }//correct resolution
                //fall through
            }//modes 1-8
            
            /*
                SCREEN 9: 640 x 350 graphics
                \A6 80 x 25 or 80 x 43 text format, 8 x 14 or 8 x 8 character box size
                \A6 64K page size, page range is 0 (64K);
                128K page size, page range is 0 (128K) or 0-1 (256K)
                \A6 16 colors assigned to 4 attributes (64K adapter memory), or
                64 colors assigned to 16 attributes (more than 64K adapter memory)
                SCREEN 10: 640 x 350 graphics, monochrome monitor only
                \A6 80 x 25 or 80 x 43 text format, 8 x 14 or 8 x 8 character box size
                \A6 128K page size, page range is 0 (128K) or 0-1 (256K)
                \A6 Up to 9 pseudocolors assigned to 4 attributes
            */
            if ((write_page->compatible_mode>=9)&&(write_page->compatible_mode<=10)){
                f=0;
                if(write_page->height==height*8) f=8;
                if(write_page->height==height*14) f=14;
                if((height==43)&&(write_page->height==350)) f=8;//?x350,8x8
                if(f){//correct resolution
                    if (write_page->font==f) return;//correct font, no change required
                    if (write_page->flags&IMG_SCREEN){
                        //delete pages 1-?
                        for(i=1;i<pages;i++){
                            if(i2=page[i]){
                                if (display_page_index==i2){display_page_index=page[0]; display_page=&img[display_page_index];}
                                if (read_page_index==i2){read_page_index=display_page_index; read_page=display_page;}
                                if (write_page_index==i2){write_page_index=display_page_index; write_page=display_page;}
                                //manual delete, freeing video pages is usually illegal
                                qb_release_img_owned_resources(i2);
                                freeimg(i2);
                            }
                        }//i
                    }
                    col=write_page->color; col2=write_page->background_color;
                    window_x1=write_page->window_x1; window_x2=write_page->window_x2; window_y1=write_page->window_y1; window_y2=write_page->window_y2;
                    imgrevert(write_page_index);
                    qbg_sub_window(window_x1,window_y1,window_x2,window_y2,1+2); write_page->clipping_or_scaling=0;
                    write_page->color=col; write_page->background_color=col2;
                    selectfont(f,write_page);
                    return;
                }//correct resolution
                //fall through
            }//modes 9,10
            
            if ((write_page->compatible_mode>=11)&&(write_page->compatible_mode<=12)){
                f=0;
                if(write_page->height==height*8) f=8;
                if(write_page->height==height*16) f=16;
                if(f){//correct resolution
                    if (write_page->font==f) return;//correct font, no change required
                    if (write_page->flags&IMG_SCREEN){
                        //delete pages 1-?
                        for(i=1;i<pages;i++){
                            if(i2=page[i]){
                                if (display_page_index==i2){display_page_index=page[0]; display_page=&img[display_page_index];}
                                if (read_page_index==i2){read_page_index=display_page_index; read_page=display_page;}
                                if (write_page_index==i2){write_page_index=display_page_index; write_page=display_page;}
                                //manual delete, freeing video pages is usually illegal
                                qb_release_img_owned_resources(i2);
                                freeimg(i2);
                            }
                        }//i
                    }
                    col=write_page->color; col2=write_page->background_color;
                    window_x1=write_page->window_x1; window_x2=write_page->window_x2; window_y1=write_page->window_y1; window_y2=write_page->window_y2;
                    imgrevert(write_page_index);
                    qbg_sub_window(window_x1,window_y1,window_x2,window_y2,1+2); write_page->clipping_or_scaling=0;
                    write_page->color=col; write_page->background_color=col2;
                    selectfont(f,write_page);
                    return;
                }//correct resolution
                //fall through
            }//modes 11,12
            
            //fall through:
            if ((height==25)||(height==50)||(height==43)){
                sub_screen_height_in_characters=height; qbg_screen(0,0,0,0,0,1);
                return;
            }
            
            goto error;
            
        }//width omitted
        
        if (!(passed&2)){//height omitted
            
            if (width<=0) goto error;
            
            if (!write_page->compatible_mode){//0
                height=write_page->height;
                f=8;
                if (height<=49) f=14;
                if (height<=42) f=16;
                if (width<=40) f++;
                if ((write_page->font==f)&&(write_page->width==width)) return;//no change
                sub_screen_height_in_characters=height; sub_screen_width_in_characters=width;
                sub_screen_font=f;
                qbg_screen(0,0,0,0,0,1);
                return;
            }//0
            
            if (((write_page->compatible_mode>=1)&&(write_page->compatible_mode<=8))||(write_page->compatible_mode==13)){
                if (write_page->width==width*8){//correct resolution
                    if (write_page->font==8) return;//correct font, no change required
                    if (write_page->flags&IMG_SCREEN){
                        //delete pages 1-?
                        for(i=1;i<pages;i++){
                            if(i2=page[i]){
                                if (display_page_index==i2){display_page_index=page[0]; display_page=&img[display_page_index];}
                                if (read_page_index==i2){read_page_index=display_page_index; read_page=display_page;}
                                if (write_page_index==i2){write_page_index=display_page_index; write_page=display_page;}
                                //manual delete, freeing video pages is usually illegal
                                qb_release_img_owned_resources(i2);
                                freeimg(i2);
                            }
                        }//i
                    }
                    col=write_page->color; col2=write_page->background_color;
                    imgrevert(write_page_index);
                    write_page->color=col; write_page->background_color=col2;
                    selectfont(8,write_page);
                    return;
                }//correct resolution
                //fall through
            }//modes 1-8
            
            /*
                SCREEN 9: 640 x 350 graphics
                \A6 80 x 25 or 80 x 43 text format, 8 x 14 or 8 x 8 character box size
                \A6 64K page size, page range is 0 (64K);
                128K page size, page range is 0 (128K) or 0-1 (256K)
                \A6 16 colors assigned to 4 attributes (64K adapter memory), or
                64 colors assigned to 16 attributes (more than 64K adapter memory)
                SCREEN 10: 640 x 350 graphics, monochrome monitor only
                \A6 80 x 25 or 80 x 43 text format, 8 x 14 or 8 x 8 character box size
                \A6 128K page size, page range is 0 (128K) or 0-1 (256K)
                \A6 Up to 9 pseudocolors assigned to 4 attributes
            */
            if ((write_page->compatible_mode>=9)&&(write_page->compatible_mode<=10)){
                f=0;
                if (write_page->width==width*8) f=8;
                if (f){//correct resolution
                    f2=fontheight[write_page->font]; if (f2>8) f=14;
                    if (write_page->font==f) return;//correct font, no change required
                    if (write_page->flags&IMG_SCREEN){
                        //delete pages 1-?
                        for(i=1;i<pages;i++){
                            if(i2=page[i]){
                                if (display_page_index==i2){display_page_index=page[0]; display_page=&img[display_page_index];}
                                if (read_page_index==i2){read_page_index=display_page_index; read_page=display_page;}
                                if (write_page_index==i2){write_page_index=display_page_index; write_page=display_page;}
                                //manual delete, freeing video pages is usually illegal
                                qb_release_img_owned_resources(i2);
                                freeimg(i2);
                            }
                        }//i
                    }
                    col=write_page->color; col2=write_page->background_color;
                    window_x1=write_page->window_x1; window_x2=write_page->window_x2; window_y1=write_page->window_y1; window_y2=write_page->window_y2;
                    imgrevert(write_page_index);
                    qbg_sub_window(window_x1,window_y1,window_x2,window_y2,1+2); write_page->clipping_or_scaling=0;
                    write_page->color=col; write_page->background_color=col2;
                    selectfont(f,write_page);
                    return;
                }//correct resolution
                //fall through
            }//modes 9,10
            
            if ((write_page->compatible_mode>=11)&&(write_page->compatible_mode<=12)){
                f=0;
                if (write_page->width==width*8) f=8;
                if (f){//correct resolution
                    f2=fontheight[write_page->font]; if (f2>8) f=16;
                    if (write_page->font==f) return;//correct font, no change required
                    if (write_page->flags&IMG_SCREEN){
                        //delete pages 1-?
                        for(i=1;i<pages;i++){
                            if(i2=page[i]){
                                if (display_page_index==i2){display_page_index=page[0]; display_page=&img[display_page_index];}
                                if (read_page_index==i2){read_page_index=display_page_index; read_page=display_page;}
                                if (write_page_index==i2){write_page_index=display_page_index; write_page=display_page;}
                                //manual delete, freeing video pages is usually illegal
                                qb_release_img_owned_resources(i2);
                                freeimg(i2);
                            }
                        }//i
                    }
                    col=write_page->color; col2=write_page->background_color;
                    window_x1=write_page->window_x1; window_x2=write_page->window_x2; window_y1=write_page->window_y1; window_y2=write_page->window_y2;
                    imgrevert(write_page_index);
                    qbg_sub_window(window_x1,window_y1,window_x2,window_y2,1+2); write_page->clipping_or_scaling=0;
                    write_page->color=col; write_page->background_color=col2;
                    selectfont(f,write_page);
                    return;
                }//correct resolution
                //fall through
            }//modes 11,12
            
            //fall through:
            if ((width==40)||(width==80)){
                sub_screen_width_in_characters=width;
                qbg_screen(0,0,0,0,0,1);
                return;
            }
            
            goto error;
            
        }//height omitted
        
        //both height & width passed
        
        if ((width<=0)||(height<=0)) goto error;
        
        if (!write_page->compatible_mode){//0
            f=8;
            if (height<=49) f=14;
            if (height<=42) f=16;
            if (width<=40) f++;
            if ((write_page->font==f)&&(write_page->width==width)&&(write_page->height==height)) return;//no change
            sub_screen_height_in_characters=height; sub_screen_width_in_characters=width;
            sub_screen_font=f;
            qbg_screen(0,0,0,0,0,1);
            return;
        }//0
        
        if (((write_page->compatible_mode>=1)&&(write_page->compatible_mode<=8))||(write_page->compatible_mode==13)){
            if ((write_page->width==width*8)&&(write_page->height==height*8)){//correct resolution
                if (write_page->font==8) return;//correct font, no change required
                if (write_page->flags&IMG_SCREEN){
                    //delete pages 1-?
                    for(i=1;i<pages;i++){
                        if(i2=page[i]){
                            if (display_page_index==i2){display_page_index=page[0]; display_page=&img[display_page_index];}
                            if (read_page_index==i2){read_page_index=display_page_index; read_page=display_page;}
                            if (write_page_index==i2){write_page_index=display_page_index; write_page=display_page;}
                            //manual delete, freeing video pages is usually illegal
                            qb_release_img_owned_resources(i2);
                            freeimg(i2);
                        }
                    }//i
                }
                col=write_page->color; col2=write_page->background_color;
                imgrevert(write_page_index);
                write_page->color=col; write_page->background_color=col2;
                selectfont(8,write_page);
                return;
            }//correct resolution
            //fall through
        }//modes 1-8
        
        /*
            SCREEN 9: 640 x 350 graphics
            \A6 80 x 25 or 80 x 43 text format, 8 x 14 or 8 x 8 character box size
            \A6 64K page size, page range is 0 (64K);
            128K page size, page range is 0 (128K) or 0-1 (256K)
            \A6 16 colors assigned to 4 attributes (64K adapter memory), or
            64 colors assigned to 16 attributes (more than 64K adapter memory)
            SCREEN 10: 640 x 350 graphics, monochrome monitor only
            \A6 80 x 25 or 80 x 43 text format, 8 x 14 or 8 x 8 character box size
            \A6 128K page size, page range is 0 (128K) or 0-1 (256K)
            \A6 Up to 9 pseudocolors assigned to 4 attributes
        */
        if ((write_page->compatible_mode>=9)&&(write_page->compatible_mode<=10)){
            f=0;
            if (write_page->width==width*8){
                if (write_page->height==height*8) f=8;
                if (write_page->height==height*14) f=14;
                if ((height==43)&&(write_page->height==350)) f=8;//?x350,8x8
            }
            if (f){//correct resolution
                if (write_page->font==f) return;//correct font, no change required
                if (write_page->flags&IMG_SCREEN){
                    //delete pages 1-?
                    for(i=1;i<pages;i++){
                        if(i2=page[i]){
                            if (display_page_index==i2){display_page_index=page[0]; display_page=&img[display_page_index];}
                            if (read_page_index==i2){read_page_index=display_page_index; read_page=display_page;}
                            if (write_page_index==i2){write_page_index=display_page_index; write_page=display_page;}
                            //manual delete, freeing video pages is usually illegal
                            qb_release_img_owned_resources(i2);
                            freeimg(i2);
                        }
                    }//i
                }
                col=write_page->color; col2=write_page->background_color;
                window_x1=write_page->window_x1; window_x2=write_page->window_x2; window_y1=write_page->window_y1; window_y2=write_page->window_y2;
                imgrevert(write_page_index);
                qbg_sub_window(window_x1,window_y1,window_x2,window_y2,1+2); write_page->clipping_or_scaling=0;
                write_page->color=col; write_page->background_color=col2;
                selectfont(f,write_page);
                return;
            }//correct resolution
            //fall through
        }//modes 9,10
        
        if ((write_page->compatible_mode>=11)&&(write_page->compatible_mode<=12)){
            f=0;
            if (write_page->width==width*8){
                
                if (write_page->height==height*8) f=8;
                
                if (write_page->height==height*16) f=16;
            }
            if (f){//correct resolution
                if (write_page->font==f) return;//correct font, no change required
                if (write_page->flags&IMG_SCREEN){
                    //delete pages 1-?
                    for(i=1;i<pages;i++){
                        if(i2=page[i]){
                            if (display_page_index==i2){display_page_index=page[0]; display_page=&img[display_page_index];}
                            if (read_page_index==i2){read_page_index=display_page_index; read_page=display_page;}
                            if (write_page_index==i2){write_page_index=display_page_index; write_page=display_page;}
                            //manual delete, freeing video pages is usually illegal
                            qb_release_img_owned_resources(i2);
                            freeimg(i2);
                        }
                    }//i
                }
                col=write_page->color; col2=write_page->background_color;
                window_x1=write_page->window_x1; window_x2=write_page->window_x2; window_y1=write_page->window_y1; window_y2=write_page->window_y2;
                imgrevert(write_page_index);
                qbg_sub_window(window_x1,window_y1,window_x2,window_y2,1+2); write_page->clipping_or_scaling=0;
                write_page->color=col; write_page->background_color=col2;
                selectfont(f,write_page);
                return;
            }//correct resolution
            //fall through
        }//modes 11,12
        
        //fall through:
        if ((width==40)||(width==80)){
            if ((height==25)||(height==50)||(height==43)){
                sub_screen_width_in_characters=width; sub_screen_height_in_characters=height;
                f=16;
                if (height==43) f=14;
                if (height==50) f=8;
                if (width==40) f++;
                sub_screen_font=f;
                qbg_screen(0,0,0,0,0,1);
                return;
            }
            
            goto error;
            
        }//WIDTH [?][,?]
        
    }//option==0
    
    if (option==2){//LPRINT
        if (passed!=1) goto error;
        if ((value1<1)||(value1>255)) goto error;
        width_lprint=value1;
        return;
    }//option==2
    
    //file/device?
    //...
    
    error:
    error(5);
    return;
}

// Drawing primitives: Rust (crates/qb-runtime/src/gfx). Shims keep the legacy
// signatures for generated code and the C++ that is not ported yet.
void pset_and_clip(int32 x,int32 y,uint32 col){
    if ((x>=write_page->view_x1)&&(x<=write_page->view_x2)&&(y>=write_page->view_y1)&&(y<=write_page->view_y2)) qb_gfx_pset(write_page,x,y,col);
}
void qb32_boxfill(float x1f,float y1f,float x2f,float y2f,uint32 col){qb_gfx_boxfill(write_page,x1f,y1f,x2f,y2f,col);}
void fast_boxfill(int32 x1,int32 y1,int32 x2,int32 y2,uint32 col){qb_gfx_fast_boxfill(write_page,x1,y1,x2,y2,col);}
void fast_line(int32 x1,int32 y1,int32 x2,int32 y2,uint32 col){qb_gfx_fast_line(write_page,x1,y1,x2,y2,col);}

void sub_line(float x1,float y1,float x2,float y2,uint32 col,int32 bf,uint32 style,int32 passed){
    if (new_error) return;
    if (int32 e=qb_gfx_line(write_page,x1,y1,x2,y2,col,bf,style,passed)) error(e);
}

void sub_paint(float x,float y,uint32 fillcol,uint32 bordercol,qbs *backgroundstr,int32 passed){
    if (new_error) return;
    if (int32 e=qb_gfx_paint(write_page,x,y,fillcol,bordercol,passed)) error(e);
}

void sub_paint(float x,float y,qbs *fillstr,uint32 bordercol,qbs *backgroundstr,int32 passed){
    if (new_error) return;
    if (qbg_text_only) {error(5); return;}
    if (int32 e=qb_gfx_paint_tile(write_page,x,y,fillstr->chr,fillstr->len,bordercol,passed)) error(e);
}

void sub_circle(double x,double y,double r,uint32 col,double start,double end,double aspect,int32 passed){
    if (new_error) return;
    if (int32 e=qb_gfx_circle(write_page,x,y,r,col,start,end,aspect,passed)) error(e);
}

uint32 point(int32 x,int32 y){return qb_gfx_point_raw(read_page,x,y);}//does not clip!

double func_point(float x,float y,int32 passed){
    int32 e=0; double v=qb_gfx_point(write_page,read_page,x,y,passed,&e);
    if (e){error(e); return 0;}
    return v;
}

void sub_pset(float x,float y,uint32 col,int32 passed){
    if (new_error) return;
    if (int32 e=qb_gfx_pset_stmt(write_page,x,y,col,passed)) error(e);
}

void sub_preset(float x,float y,uint32 col,int32 passed){
    if (new_error) return;
    if (int32 e=qb_gfx_preset_stmt(write_page,x,y,col,passed)) error(e);
}


int32 img_printchr=0;
int32 img_printchr_i;
int32 img_printchr_x;
int32 img_printchr_y;
char *img_printchr_offset;

void printchr(int32 character,int32 cell_span){
    static uint32 x,x2,y,y2,w,h,z,z2,z3,a,a2,a3,color,background_color,f;
    static uint32 *lp;
    static uint8 *cp;
    static img_struct *im;
    
    im=write_page;
    color=im->color;
    background_color=im->background_color;
    
    
    if (im->text){
        const int32 cell_index=(im->cursor_y-1)*im->width+im->cursor_x-1;
        im->offset[cell_index<<1]=qb_unicode_to_screen_byte(character);
        im->offset[(cell_index<<1)+1]=(color&0xF)+background_color*16+(color&16)*8;
        if (cell_span > 1 && cell_index + 1 < im->width * im->height){
            im->offset[(cell_index+1)<<1]=32;
            im->offset[((cell_index+1)<<1)+1]=(color&0xF)+background_color*16+(color&16)*8;
        }
        qb_store_text_cell_utf8(im,cell_index,character,cell_span);
        return;
    }
    
    //precalculations
    
    f=im->font;
    x=fontwidth[f]; if (x) x*=(im->cursor_x-1); else x=im->cursor_x-1;
    y=(im->cursor_y-1)*fontheight[f];
    h=fontheight[f];
    if ((fontflags[f]&32)==0) character&=255;//unicodefontsupport
    
    //if (mode==1) img[i].print_mode=3;//fill
    //if (mode==2) img[i].print_mode=1;//keep
    //if (mode==3) img[i].print_mode=2;//only
    
    if (f>=32){//custom font
        
        //8-bit / alpha-disabled 32-bit / dont-blend(alpha may still be applied)
        if ((im->bytes_per_pixel==1)||((im->bytes_per_pixel==4)&&(im->alpha_disabled))||(fontflags[f]&8)){
            
            //render character
            static int32 ok;
            static uint8 *rt_data;
            static int32 rt_w,rt_h,rt_pre_x,rt_post_x;
            //int32 FontRenderTextASCII(int32 i,uint8*codepoint,int32 codepoints,int32 options,
            //                          uint8**out_data,int32*out_x,int32 *out_y,int32*out_x_pre_increment,int32*out_x_post_increment){
            ok=FontRenderTextASCII(font[f],(uint8*)&character,1,1,
            &rt_data,&rt_w,&rt_h,&rt_pre_x,&rt_post_x);
            if (!ok) return;
            
            w=rt_w;
            
            switch(im->print_mode){
                case 3:
                for (y2=0;y2<h;y2++){
                    cp=rt_data+y2*w;
                    for (x2=0;x2<w;x2++){
                        if (*cp++) pset(x+x2,y+y2,color); else pset(x+x2,y+y2,background_color);
                    }}
                    break;
                    case 1:
                    for (y2=0;y2<h;y2++){
                        cp=rt_data+y2*w;
                        for (x2=0;x2<w;x2++){
                            if (*cp++) pset(x+x2,y+y2,color);
                        }}
                        break;
                        case 2:
                        for (y2=0;y2<h;y2++){
                            cp=rt_data+y2*w;
                            for (x2=0;x2<w;x2++){
                                if (!(*cp++)) pset(x+x2,y+y2,background_color);
                            }}
                            break;
                            default:
                            break;
            }
            
            free(rt_data);
            return;
        }//1-8 bit
        //assume 32-bit blended
        
        a=(color>>24)+1; a2=(background_color>>24)+1;
        z=color&0xFFFFFF; z2=background_color&0xFFFFFF;
        
        //render character
        static int32 ok;
        static uint8 *rt_data;
        static int32 rt_w,rt_h,rt_pre_x,rt_post_x;
        //int32 FontRenderTextASCII(int32 i,uint8*codepoint,int32 codepoints,int32 options,
        //                          uint8**out_data,int32*out_x,int32 *out_y,int32*out_x_pre_increment,int32*out_x_post_increment){
        ok=FontRenderTextASCII(font[f],(uint8*)&character,1,0,
        &rt_data,&rt_w,&rt_h,&rt_pre_x,&rt_post_x);
        if (!ok) return;
        
        w=rt_w;
        
        switch(im->print_mode){
            case 3:
            
            static float r1,g1,b1,alpha1,r2,g2,b2,alpha2;
            alpha1=(color>>24)&255; r1=(color>>16)&255; g1=(color>>8)&255; b1=color&255;
            alpha2=(background_color>>24)&255; r2=(background_color>>16)&255; g2=(background_color>>8)&255; b2=background_color&255;
            static float dr,dg,db,da;
            dr=r2-r1;
            dg=g2-g1;
            db=b2-b1;
            da=alpha2-alpha1;
            static float cw;//color weight multiplier, avoids seeing black when transitioning from RGBA(?,?,?,255) to RGBA(0,0,0,0)
            if (alpha1) cw=alpha2/alpha1; else cw=100000;
            static float d;
            
            for (y2=0;y2<h;y2++){
                cp=rt_data+y2*w;
                for (x2=0;x2<w;x2++){
                    
                    d=*cp++;
                    d=255-d;
                    d/=255.0;
                    static float r3,g3,b3,alpha3;
                    alpha3=alpha1+da*d;
                    d*=cw; if (d>1.0) d=1.0;
                    r3=r1+dr*d;
                    g3=g1+dg*d;
                    b3=b1+db*d;
                    static int32 r4,g4,b4,alpha4;
                    r4=qbr_float_to_long(r3);
                    g4=qbr_float_to_long(g3);
                    b4=qbr_float_to_long(b3);
                    alpha4=qbr_float_to_long(alpha3);
                    pset(x+x2,y+y2,b4+(g4<<8)+(r4<<16)+(alpha4<<24));
                    
                }}
                break;
                case 1:
                for (y2=0;y2<h;y2++){
                    cp=rt_data+y2*w;
                    for (x2=0;x2<w;x2++){
                        z3=*cp++;
                        if (z3) pset(x+x2,y+y2,((z3*a)>>8<<24)+z);
                    }}
                    break;
                    case 2:
                    for (y2=0;y2<h;y2++){
                        cp=rt_data+y2*w;
                        for (x2=0;x2<w;x2++){
                            z3=*cp++;
                            if (z3!=255) pset(x+x2,y+y2,(((255-z3)*a2)>>8<<24)+z2);
                        }}
                        break;
                        default:
                        break;
        }
        free(rt_data);
        return;
    }//custom font
    
    
    //default fonts (8x8, 8x14, 8x16): Rust (qb-runtime gfx/text.rs)
    qb_gfx_printchr(im,character);
    return;
}//printchr





int32 chrwidth(uint32 character){
    //Note: Only called by qbs_print()
    //      Supports "UNICODE" _LOADFONT option
    static int32 w;
    static img_struct *im;
    im=write_page;
    if (w=fontwidth[im->font]) return w;
    
    //Custom font
    static int32 render_option,f;
    static int32 ok;
    static uint8 *rt_data;
    static int32 rt_w,rt_h,rt_pre_x,rt_post_x;
    
    f=im->font;
    
    render_option=0;
    //8-bit / alpha-disabled 32-bit / dont-blend(alpha may still be applied)
    if ((im->bytes_per_pixel==1)||((im->bytes_per_pixel==4)&&(im->alpha_disabled))||(fontflags[f]&8)){
        render_option=1;
    }
    
    if ((fontflags[f]&32)){//UNICODE character
        ok=FontRenderTextUTF32(font[f],(uint32*)&character,1,render_option,
        &rt_data,&rt_w,&rt_h,&rt_pre_x,&rt_post_x);
        }else{//ASCII character
        character&=255;
        ok=FontRenderTextASCII(font[f],(uint8*)&character,1,render_option,
        &rt_data,&rt_w,&rt_h,&rt_pre_x,&rt_post_x);
    }
    if (!ok) return 0;
    free(rt_data);
    return rt_w;
    
}//chrwidth


void newline(){
    static uint32 *lp;
    static uint16 *sp;
    static int32 z,z2;
    
    //move cursor to new line
    write_page->cursor_y++; write_page->cursor_x=1;
    
    //scroll up screen if necessary
    if (write_page->cursor_y>write_page->bottom_row){
        
        if (lprint){
            sub__printimage(lprint_image);
            sub_cls(NULL,15,2);
            lprint_buffered=0;
            return;
        }
        
        if (write_page->text){
            //text
            //move lines up
            memmove(
            write_page->offset+(write_page->top_row-1)*2*write_page->width,
            write_page->offset+ write_page->top_row   *2*write_page->width,
            (write_page->bottom_row-write_page->top_row)*2*write_page->width
            );
            {
                const int32 image_index=qb_img_index(write_page);
                if (image_index >= 0 && image_index < nimg && image_index < qb_text_utf8_cells.size()){
                    std::vector<std::string> &cells=qb_text_utf8_cells[image_index];
                    std::vector<uint8> &cell_state=qb_text_utf8_cell_state[image_index];
                    if (!cells.empty()){
                        const int32 row_width=write_page->width;
                        const int32 dst=(write_page->top_row-1)*row_width;
                        const int32 src=write_page->top_row*row_width;
                        const int32 moved=(write_page->bottom_row-write_page->top_row)*row_width;
                        for (int32 cell=0;cell<moved;cell++){
                            cells[dst+cell]=cells[src+cell];
                            cell_state[dst+cell]=cell_state[src+cell];
                        }
                        qb_clear_text_utf8_cells_range(image_index,(write_page->bottom_row-1)*row_width,row_width);
                    }
                }
            }
            //erase bottom line
            z2=(write_page->color&0xF)+(write_page->background_color&7)*16+(write_page->color&16)*8;
            z2<<=8;
            z2+=32;
            sp=((uint16*)(write_page->offset+(write_page->bottom_row-1)*2*write_page->width));
            z=write_page->width;
            while(z--) *sp++=z2;
            }else{
            //graphics
            //move lines up
            memmove(
            write_page->offset+(write_page->top_row-1)*write_page->bytes_per_pixel*write_page->width*fontheight[write_page->font],
            write_page->offset+ write_page->top_row   *write_page->bytes_per_pixel*write_page->width*fontheight[write_page->font],
            (write_page->bottom_row-write_page->top_row)*write_page->bytes_per_pixel*write_page->width*fontheight[write_page->font]
            );
            //erase bottom line
            if (write_page->bytes_per_pixel==1){
                memset(write_page->offset+(write_page->bottom_row-1)*write_page->width*fontheight[write_page->font],write_page->background_color,write_page->width*fontheight[write_page->font]);
                }else{
                //assume 32-bit
                z2=write_page->background_color;
                lp=write_page->offset32+(write_page->bottom_row-1)*write_page->width*fontheight[write_page->font];
                z=write_page->width*fontheight[write_page->font];
                while(z--) *lp++=z2;
            }
        }//graphics
        write_page->cursor_y=write_page->bottom_row;
    }//scroll up
    
}//newline

void makefit(qbs *text){
    static int32 w,x,x2,x3;
    if (write_page->holding_cursor) return;
    if (write_page->cursor_x!=1){//if already at left-most, nothing more can be done
        if (write_page->text){
            if ((write_page->cursor_x+text->len-1)>write_page->width) newline();
            }else{
            w=func__printwidth(text,NULL,NULL);
            x=fontwidth[write_page->font]; if (!x) x=1; x=x*(write_page->cursor_x-1);
            if ((x+w)>write_page->width) newline();
        }
    }
}

void lprint_makefit(qbs *text){
    //stub
    //makefit(text);
}

void tab(){
    static int32 x,x2,w;
    
    //tab() on a held-cursor only sets the cursor to the left hand position of the next line
    if (write_page->holding_cursor){
        newline(); write_page->holding_cursor=0;
        return;
    }

    #ifdef QBNex_WINDOWS //if Windows console
        if (write_page->console){
            if (func_pos(0)>write_page->width-10){
                printf("\n");
                return;
            }else{
                do {
                    printf(" ");
                }while(func_pos(0) % 10!=0);
            }
            return;
        }
    #endif
    
    //text
    if (write_page->text){
        qbs_print(singlespace,0);
        text:
        if (write_page->cursor_x!=1){
            if (((write_page->cursor_x-1)%14)||(write_page->cursor_x>(write_page->width-13))){
                if (write_page->cursor_x<write_page->width){qbs_print(singlespace,0); goto text;}
            }
        }//!=1
        return;
    }
    
    x=fontwidth[write_page->font]; 
    if (!x){
        
        //variable width
        x=write_page->cursor_x-1;
        x2=(x/112+1)*112;//next position
        if (x2>=write_page->width){//it doesn't fit on line
            //box fill x to end of line with background color
            fast_boxfill(x,(write_page->cursor_y-1)*fontheight[write_page->font],write_page->width-1,write_page->cursor_y*fontheight[write_page->font]-1,write_page->background_color);
            newline();
            }else{//fits on line
            //box fill x to x2-1 with background color
            fast_boxfill(x,(write_page->cursor_y-1)*fontheight[write_page->font],x2-1,write_page->cursor_y*fontheight[write_page->font]-1,write_page->background_color);
            write_page->cursor_x=x2;
        }
        
        
        }else{
        
        //fixed width
        w=write_page->width/x;
        
        qbs_print(singlespace,0);
        fixwid:
        if (write_page->cursor_x!=1){
            if (((write_page->cursor_x-1)%14)||(write_page->cursor_x>(w-13))){
                if (write_page->cursor_x<w){qbs_print(singlespace,0); goto fixwid;}
            }
        }//!=1
        
    }
    return;
}

int32 func_lpos(int32 lpt){
    //lpt values: 0 = LPT1, 1 = LPT1, 2 = LPT2, 3 = LPT3
    if ((lpt<0)||(lpt>3)){error(5); return 0;}
    return lpos;
}

void qbs_lprint(qbs* str,int32 finish_on_new_line){
    static int32 old_dest;
    while (lprint_locked) Sleep(64);
    lprint=1;
    old_dest=func__dest();
    if (!lprint_image){
        lprint_image=func__newimage(640,960,13,1);
        sub__dest(lprint_image);
        sub_cls(NULL,15,2);
        sub__font(16,NULL,0);
        qbg_sub_color(0,15,NULL,3);
        qbg_sub_view_print(1,60,1);
        }else{
        sub__dest(lprint_image);
    }
    lprint_buffered=1;
    lprint_last=func_timer(0.001,1);
    qbs_print(str,finish_on_new_line);
    sub__dest(old_dest);
    lprint=0;
}

int32 no_control_characters=0;
int32 no_control_characters2=0;

void sub__controlchr(int32 onoff){
    if (onoff==2) no_control_characters2=1; else no_control_characters2=0;
}

int32 func__controlchr () {
    return -no_control_characters2;
}

void qbs_print(qbs* str,int32 finish_on_new_line){
    if (new_error) return;
    int32 i,i2,entered_new_line,x,x2,y,y2,z,z2,w,cell_span;
    entered_new_line=0;
    static uint32 character;
    
    if (write_page->console){
        qb_write_utf8(reinterpret_cast<const char*>(str->chr), str->len, finish_on_new_line);
        return;
    }
    
    /*
        if (!str->len){
        if (!newline) return;//no action required
        if (write_page->holding_cursor){//extra CR required before return
        write_page->holding_cursor=0;
        i=-1;
        write_page->cursor_x++;
        goto print_unhold_cursor;
        }
        }
        
        if (!str->len) goto null_length;
        
        if (write_page->holding_cursor){
        write_page->holding_cursor=0;
        i=-1;
        write_page->cursor_x++;
        goto print_unhold_cursor;
        }
    */
    
    //holding cursor?
    if (write_page->holding_cursor){
        if (str->len){
            write_page->holding_cursor=0;
            newline();
            }else{
            //null length print string
            if (finish_on_new_line) write_page->holding_cursor=0;//new line will be entered automatically
        }
    }
    
    
    for (i=0;i<str->len;i++){
        
        character=(uint8)str->chr[i];
        
        if (write_page->text){
            const int32 bytes_used=qb_decode_utf8_codepoint((const uint8*)&str->chr[i],str->len-i,&character);
            if (!bytes_used) break;
            i+=bytes_used-1;
        }else if (fontflags[write_page->font]&32){//unicode font
            if (i>(str->len-4)) break;//not enough data for a utf32 encoding
            character=*((int32*)(&str->chr[i]));
            i+=3;
        }
        
        if (lprint) lprint_buffered=1;
        entered_new_line=0;//beginning a new line was the last action (so don't add a new one)
        
        
        
        //###special characters
        
        if (no_control_characters||no_control_characters2) goto skip_control_characters;
        
        if (character==28){
            //advance one cursor position
            if (lprint){
                if (lpos<width_lprint) lpos++;
            }
            //can cursor advance?
            if (write_page->cursor_y>=write_page->bottom_row){
                if (write_page->text){
                    if (write_page->cursor_x>=write_page->width) goto skip;
                    }else{
                    if (fontwidth[write_page->font]){
                        if (write_page->cursor_x>=(write_page->width/fontwidth[write_page->font])) goto skip;
                        }else{
                        if (write_page->cursor_x>=write_page->width) goto skip;
                    }
                } 
            }
            write_page->cursor_x++;
            if (write_page->text){
                if (write_page->cursor_x>write_page->width){write_page->cursor_y++; write_page->cursor_x=1;}
                }else{
                if (fontwidth[write_page->font]){
                    if (write_page->cursor_x>(write_page->width/fontwidth[write_page->font])){write_page->cursor_y++; write_page->cursor_x=1;}
                    }else{
                    if (write_page->cursor_x>write_page->width){write_page->cursor_y++; write_page->cursor_x=1;}
                }
            } 
            goto skip;
        }
        
        if (character==29){
            //go back one cursor position
            if (lprint){
                if (lpos>1) lpos--;
            }
            //can cursor go back?
            if ((write_page->cursor_y==write_page->top_row)||(write_page->cursor_y>write_page->bottom_row)){
                if (write_page->cursor_x==1) goto skip;
            }
            write_page->cursor_x--;
            if (write_page->cursor_x<1){
                write_page->cursor_y--;
                if (write_page->text){
                    write_page->cursor_x=write_page->width;
                    }else{
                    if (fontwidth[write_page->font]){
                        write_page->cursor_x=write_page->width/fontwidth[write_page->font];
                        }else{
                        write_page->cursor_x=write_page->width;
                    }
                } 
            }
            goto skip;
        }
        
        if (character==30){
            //previous row, same column
            //no change if cursor not within view print boundries
            if ((write_page->cursor_y>write_page->top_row)&&(write_page->cursor_y<=write_page->bottom_row)){
                write_page->cursor_y--;
            }
            goto skip;
        }
        
        if (character==31){
            //next row, same column
            //no change if cursor not within view print boundries
            if ((write_page->cursor_y>=write_page->top_row)&&(write_page->cursor_y<write_page->bottom_row)){
                write_page->cursor_y++;
            }
            goto skip;
        }
        
        if (character==12){//aka form feed
            if (lprint){sub__printimage(lprint_image); lprint_buffered=0;}
            //clears text viewport
            //clears bottom row
            //moves cursor to top-left of text viewport
            sub_cls(NULL,NULL,0);
            if (lprint) lpos=1;
            goto skip;
        }
        
        if (character==11){
            write_page->cursor_x=1; write_page->cursor_y=write_page->top_row;
            if (lprint) lpos=1;
            goto skip;
        }
        
        if (character==9){
            //moves to next multiple of 8 (always advances at least one space)
            if (!fontwidth[write_page->font]){
                //variable width!
                x=write_page->cursor_x-1;
                x2=(x/64+1)*64;//next position
                if (x2>=write_page->width){//it doesn't fit on line
                    //box fill x to end of line with background color
                    fast_boxfill(x,(write_page->cursor_y-1)*fontheight[write_page->font],write_page->width-1,write_page->cursor_y*fontheight[write_page->font]-1,write_page->background_color);
                    newline();
                    entered_new_line=1;
                    }else{//fits on line
                    //box fill x to x2-1 with background color
                    fast_boxfill(x,(write_page->cursor_y-1)*fontheight[write_page->font],x2-1,write_page->cursor_y*fontheight[write_page->font]-1,write_page->background_color);
                    write_page->cursor_x=x2;
                }
                goto skip;
                }else{
                if (write_page->cursor_x%8){//next cursor position not a multiple of 8
                    i--;//more spaces will be required
                }
                character=32;//override character 9
            }
        }//9
        
        if (character==7){
            //qbnex_generatesound(783.99,0.2,0);
            Sleep(250);
            goto skip;
        }
        
        if ((character==10)||(character==13)){
            newline();
            if (lprint) lpos=1;
            //note: entered_new_line not set because these carriage returns compound on each other
            goto skip;
        }
        
        skip_control_characters:
        
        if (write_page->text && qb_is_zero_width_codepoint(character)){
            int32 previous_cell=-1;
            if (write_page->cursor_x > 1){
                previous_cell=(write_page->cursor_y-1)*write_page->width+write_page->cursor_x-2;
            }else if (write_page->cursor_y > write_page->top_row){
                previous_cell=(write_page->cursor_y-2)*write_page->width+write_page->width-1;
            }
            if (previous_cell >= 0 && qb_append_text_cell_utf8(write_page,previous_cell,character)){
                goto skip;
            }
        }

        cell_span=1;
        if (write_page->text && qb_is_wide_codepoint(character)) cell_span=2;
        if (write_page->text && cell_span > 1){
            if ((write_page->cursor_x + cell_span - 1) > write_page->width){
                newline();
            }
        }
        
        //###check if character fits on line, if not move to next line
        //(only applies to non-fixed width fonts)
        if (!fontwidth[write_page->font]){//unpredictable width
            w=chrwidth(character);
            if ((write_page->cursor_x+w)>write_page->width){
                newline();
                //entered_new_line not set, a character will follow
            }
        }
        
        //###print the character
        printchr(character,cell_span);
        
        //###advance lpos, begin new line if necessary
        if (lprint){
            lpos++;
            if (lpos>width_lprint){
                newline();
                entered_new_line=1;
                lpos=1;
                goto skip;//skip cursor advancement and checking because new line entered
            }
        }
        
        //###advance cursor
        if (fontwidth[write_page->font]){
            write_page->cursor_x+=cell_span;
            }else{
            write_page->cursor_x+=w;
        }
        
        //###check if another character could fit at cursor_x's location
        if (write_page->compatible_mode){//graphics
            x=fontwidth[write_page->font]; if (!x) x=1;
            x2=x*(write_page->cursor_x-1);
            if (x2>(write_page->width-x)){
                if (!finish_on_new_line){
                    if (i==(str->len-1)){//last character
                        //move horizontal cursor back to right-most valid position
                        write_page->cursor_x=write_page->width/x;
                        write_page->holding_cursor=1;
                        goto held_cursor;
                    }
                }
                newline();
                entered_new_line=1;
            }
            }else{//text
            if (write_page->cursor_x>write_page->width){
                if (!finish_on_new_line){
                    if (i==(str->len-1)){//last character
                        write_page->cursor_x--;//move horizontal cursor back to right-most valid position
                        write_page->holding_cursor=1;
                        goto held_cursor;
                    }
                }
                newline();
                entered_new_line=1;
            }
        }
        held_cursor:
        
        skip:;
        
        /*
            tabbing1:
            
            
            write_page->cursor_x++;
            
            
            
            //hold cursor?
            if (write_page->cursor_x>qbg_width_in_characters){//past last x position
            if (!newline){//don't need a new line
            if (i==(str->len-1)){//last character
            write_page->cursor_x--;
            write_page->holding_cursor=1;
            goto hold_cursor;
            }
            }
            }
            
            
            
            
            qbs_print_skipchar:;
            
            print_unhold_cursor:
            
            if (write_page->cursor_x>qbg_width_in_characters){
            qbs_print_newline:
            newlineadded=1;
            
            if (write_page->cursor_y==qbg_height_in_characters) write_page->cursor_y=qbg_bottom_row;
            
            write_page->cursor_y++;
            write_page->cursor_x=1;
            
            
            
            if (write_page->cursor_y>qbg_bottom_row){
            //move screen space within view print up 1 row
            //if (qbg_mode==13){
            
            ///memmove(&cmem[655360+(qbg_top_row-1)*2560],&cmem[655360+qbg_top_row*2560],(qbg_bottom_row-qbg_top_row)*2560);
            ///memset(&cmem[655360+(qbg_bottom_row-1)*2560],0,2560);
            if (qbg_text_only){
            
            memmove(qbg_active_page_offset+(qbg_top_row-1)*qbg_width_in_characters*2,
            qbg_active_page_offset+(qbg_top_row)*qbg_width_in_characters*2,
            (qbg_bottom_row-qbg_top_row)*qbg_width_in_characters*2);
            for (i2=0;i2<qbg_width_in_characters;i2++){
            qbg_active_page_offset[(qbg_bottom_row-1)*qbg_width_in_characters*2+i2*2]=32;
            qbg_active_page_offset[(qbg_bottom_row-1)*qbg_width_in_characters*2+i2*2+1]=7;
            
            }
            
            }else{
            memmove(qbg_active_page_offset+(qbg_top_row-1)*qbg_bytes_per_pixel*qbg_width*qbg_character_height,
            qbg_active_page_offset+qbg_top_row*qbg_bytes_per_pixel*qbg_width*qbg_character_height,
            (qbg_bottom_row-qbg_top_row)*qbg_bytes_per_pixel*qbg_width*qbg_character_height);
            memset(qbg_active_page_offset+(qbg_bottom_row-1)*qbg_bytes_per_pixel*qbg_width*qbg_character_height,0,qbg_bytes_per_pixel*qbg_width*qbg_character_height);
            }
            
            
            
            
            write_page->cursor_y=qbg_bottom_row;
            }
            
            
            
            }
        */
        
    }//i
    
    null_length:
    if (finish_on_new_line&&(!entered_new_line)) newline();
    if (lprint){
        if (finish_on_new_line) lpos=1;
    }
    
    
    /*
        null_length:
        
        //begin new line?
        if (newline&&(!newlineadded)) {newline=0; goto qbs_print_newline;}
        
        //hold cursor
        hold_cursor:
    */
    
    return;
    
}

template <typename T> static T qbs_cleanup(uint32 base,T passvalue){
    if (qbs_tmp_list_nexti>base) qb_qbs_cleanup(base);//clear any temp. strings created (Rust, qbs.rs)
    return passvalue;
}

void qbg_sub_window(float x1,float y1,float x2,float y2,int32 passed){
    if (new_error) return;
    if (int32 e=qb_gfx_window(write_page,x1,y1,x2,y2,passed)) error(e);
}



void qbg_sub_view_print(int32 topline,int32 bottomline,int32 passed){
    if (new_error) return;
    
    static int32 maxrows;
    maxrows=write_page->height; if (!write_page->text) maxrows/=fontheight[write_page->font];
    
    if (!passed){//topline and bottomline not passed
        write_page->top_row=1; write_page->bottom_row=maxrows;
        write_page->cursor_y=1; write_page->cursor_x=1;
        write_page->holding_cursor=0;
        return;
    }
    
    if (topline<=0) goto error;
    if (topline>maxrows) goto error;
    if (bottomline<topline) goto error;
    if (bottomline>maxrows) goto error;
    
    write_page->top_row=topline;
    write_page->bottom_row=bottomline;
    write_page->cursor_y=write_page->top_row;
    write_page->cursor_x=1;
    write_page->holding_cursor=0;
    return;
    
    error:
    error(5);
    return;
}

void qbg_sub_view(int32 x1,int32 y1,int32 x2,int32 y2,int32 fillcolor,int32 bordercolor,int32 passed){
    if (new_error) return;
    if (int32 e=qb_gfx_view(write_page,x1,y1,x2,y2,fillcolor,bordercolor,passed)) error(e);
}


void qbg_sub_locate(int32 row,int32 column,int32 cursor,int32 start,int32 stop,int32 passed);
void sub_cls(int32 method,uint32 use_color,int32 passed){
    if (new_error) return;
    static int32 characters,i;
    static uint16 *sp;
    static uint16 clearvalue;
    
	if (write_page->console){ 
		#ifdef QBNex_WINDOWS
			system("cls"); //lazy but works
			qbg_sub_locate(1, 1, 0, 0, 0, 3); //is this really necessary?
		#else
			if (passed&2) qbg_sub_color(0, use_color, 0, 2);
			cout<<"\033[2J";
			qbg_sub_locate(1, 1, 0, 0, 0, 3);
		#endif
		return;
	}

    //validate
    if (passed&2){
        if (write_page->bytes_per_pixel!=4){
            if (use_color>write_page->mask) goto error;
        }
        }else{
        use_color=write_page->background_color;
    }
    
    if (passed&1){
        if ((method>2)||(method<0)) goto error;
    }
    
    
    
    //all CLS methods reset the cursor position
    write_page->cursor_y=write_page->top_row;
    write_page->cursor_x=1;
    
    //all CLS methods reset DRAW attributes
    write_page->draw_ta=0.0; write_page->draw_scale=1.0;
    
    if (write_page->text){
        //precalculate a (int16) value which can be used to clear the screen
        clearvalue=(write_page->color&0xF)+(use_color&7)*16+(write_page->color&16)*8;
        clearvalue<<=8;
        clearvalue+=32;
    }
    
    if ((passed&1)==0){//no method specified
        //video mode: clear only graphics viewport
        //text mode: clear text view port AND the bottom line
        if (write_page->text){
            //text view port
            characters=write_page->width*(write_page->bottom_row-write_page->top_row+1);
            sp=(uint16*)&write_page->offset[(write_page->top_row-1)*write_page->width*2];
            for (i=0;i<characters;i++){sp[i]=clearvalue;}
            qb_clear_text_utf8_cells_range(qb_img_index(write_page),(write_page->top_row-1)*write_page->width,characters);
            //bottom line
            characters=write_page->width;
            sp=(uint16*)&write_page->offset[(write_page->height-1)*write_page->width*2];
            for (i=0;i<characters;i++){sp[i]=clearvalue;}
            qb_clear_text_utf8_cells_range(qb_img_index(write_page),(write_page->height-1)*write_page->width,characters);
            key_display_redraw=1; key_update();
            return;
            }else{//graphics
            //graphics view port
            if (write_page->bytes_per_pixel==1){//8-bit
                if (write_page->clipping_or_scaling){
                    qb32_boxfill(write_page->window_x1,write_page->window_y1,write_page->window_x2,write_page->window_y2,use_color);
                    }else{//fast method (no clipping/scaling)
                    memset(write_page->offset,use_color,write_page->width*write_page->height);
                }
                }else{//32-bit
                i=write_page->alpha_disabled; write_page->alpha_disabled=1;  
                if (write_page->clipping_or_scaling){
                    qb32_boxfill(write_page->window_x1,write_page->window_y1,write_page->window_x2,write_page->window_y2,use_color);
                    }else{//fast method (no clipping/scaling)
                    fast_boxfill(0,0,write_page->width-1,write_page->height-1,use_color);
                }
                write_page->alpha_disabled=i;
            }
        }
        
        if (write_page->clipping_or_scaling==2){
            write_page->x=((float)(write_page->view_x2-write_page->view_x1+1))/write_page->scaling_x/2.0f+write_page->scaling_offset_x;
            write_page->y=((float)(write_page->view_y2-write_page->view_y1+1))/write_page->scaling_y/2.0f+write_page->scaling_offset_y;
            }else{
            write_page->x=((float)(write_page->view_x2-write_page->view_x1+1))/2.0f;
            write_page->y=((float)(write_page->view_y2-write_page->view_y1+1))/2.0f;
        }
        
        key_display_redraw=1; key_update();
        return;
    }
    
    if (method==0){//clear everything
        if (write_page->text){
            characters=write_page->height*write_page->width;
            sp=(uint16*)write_page->offset;
            for (i=0;i<characters;i++){sp[i]=clearvalue;}
            qb_clear_text_utf8_cells_range(qb_img_index(write_page),0,characters);
            key_display_redraw=1; key_update();
            return;
            }else{
            if (write_page->bytes_per_pixel==1){
                memset(write_page->offset,use_color,write_page->width*write_page->height);
                }else{ 
                i=write_page->alpha_disabled; write_page->alpha_disabled=1;  
                fast_boxfill(0,0,write_page->width-1,write_page->height-1,use_color);
                write_page->alpha_disabled=i;
            }
        }
        
        if (write_page->clipping_or_scaling==2){
            write_page->x=((float)(write_page->view_x2-write_page->view_x1+1))/write_page->scaling_x/2.0f+write_page->scaling_offset_x;
            write_page->y=((float)(write_page->view_y2-write_page->view_y1+1))/write_page->scaling_y/2.0f+write_page->scaling_offset_y;
            }else{
            write_page->x=((float)(write_page->view_x2-write_page->view_x1+1))/2.0f;
            write_page->y=((float)(write_page->view_y2-write_page->view_y1+1))/2.0f;
        }
        
        key_display_redraw=1; key_update();
        return;
    }
    
    if (method==1){//ONLY clear the graphics viewport
        if (write_page->text) return;
        //graphics view port
        if (write_page->bytes_per_pixel==1){//8-bit
            if (write_page->clipping_or_scaling){
                qb32_boxfill(write_page->window_x1,write_page->window_y1,write_page->window_x2,write_page->window_y2,use_color);
                }else{//fast method (no clipping/scaling)
                memset(write_page->offset,use_color,write_page->width*write_page->height);
            }
            }else{//32-bit
            i=write_page->alpha_disabled; write_page->alpha_disabled=1;  
            if (write_page->clipping_or_scaling){
                qb32_boxfill(write_page->window_x1,write_page->window_y1,write_page->window_x2,write_page->window_y2,use_color);
                }else{//fast method (no clipping/scaling)
                fast_boxfill(0,0,write_page->width-1,write_page->height-1,use_color);
            }
            write_page->alpha_disabled=i;
        }
        
        if (write_page->clipping_or_scaling==2){
            write_page->x=((float)(write_page->view_x2-write_page->view_x1+1))/write_page->scaling_x/2.0f+write_page->scaling_offset_x;
            write_page->y=((float)(write_page->view_y2-write_page->view_y1+1))/write_page->scaling_y/2.0f+write_page->scaling_offset_y;
            }else{
            write_page->x=((float)(write_page->view_x2-write_page->view_x1+1))/2.0f;
            write_page->y=((float)(write_page->view_y2-write_page->view_y1+1))/2.0f;
        }
        
        key_display_redraw=1; key_update();
        return;
    }
    
    if (method==2){//ONLY clear the VIEW PRINT range text viewport
        if (write_page->text){
            //text viewport
            characters=write_page->width*(write_page->bottom_row-write_page->top_row+1);
            sp=(uint16*)&write_page->offset[(write_page->top_row-1)*write_page->width*2];
            for (i=0;i<characters;i++){sp[i]=clearvalue;}
            qb_clear_text_utf8_cells_range(qb_img_index(write_page),(write_page->top_row-1)*write_page->width,characters);
            //should not and does not redraw KEY bar
            return;
            }else{
            //text viewport
            if (write_page->bytes_per_pixel==1){//8-bit
                memset(&write_page->offset[write_page->width*fontheight[write_page->font]*(write_page->top_row-1)],use_color,write_page->width*fontheight[write_page->font]*(write_page->bottom_row-write_page->top_row+1));
                }else{//32-bit
                i=write_page->alpha_disabled; write_page->alpha_disabled=1;  
                fast_boxfill(0,fontheight[write_page->font]*(write_page->top_row-1),write_page->width-1,fontheight[write_page->font]*write_page->bottom_row-1,use_color);
                write_page->alpha_disabled=i;
            }
            //should not and does not redraw KEY bar
            return;
        }
    }
    
    return;
    error:
    error(5);
    return;
}


void qbg_sub_locate(int32 row,int32 column,int32 cursor,int32 start,int32 stop,int32 passed){
    static int32 h,w,i;
    if (new_error) return;

	if (write_page->console){
		#ifdef QBNex_WINDOWS //If trying to locate with windows console
			CONSOLE_SCREEN_BUFFER_INFO cl_bufinfo;
			SECURITY_ATTRIBUTES SecAttribs = {sizeof(SECURITY_ATTRIBUTES), 0, 1};
			HANDLE cl_conout = CreateFileA("CONOUT$", GENERIC_READ | GENERIC_WRITE, FILE_SHARE_READ | FILE_SHARE_WRITE, & SecAttribs, OPEN_EXISTING, 0, 0);
			GetConsoleScreenBufferInfo(cl_conout, & cl_bufinfo);
			if (column==0)column=cl_bufinfo.dwCursorPosition.X + 1;
			if (row==0)row=cl_bufinfo.dwCursorPosition.Y + 1;
			COORD pos = {column-1, row-1};
			HANDLE output = GetStdHandle (STD_OUTPUT_HANDLE);
			SetConsoleCursorPosition(output, pos);
		#else
            if (passed & 1 && passed & 2){
                printf("\033[%d;%dH", row, column);
                return;
            }
            if (passed & 1){
                printf("\033[%dd", row);
                return;
            }
            if (passed & 2){
                printf("\033[%d;%dG", column);
                return;
            }
        #endif
        return;
	}

    //calculate height & width in characters
    if (write_page->compatible_mode){
        h=write_page->height/fontheight[write_page->font];
        if (fontwidth[write_page->font]){
            w=write_page->width/fontwidth[write_page->font];
            }else{
            w=write_page->width;
        }
        }else{
        h=write_page->height;
        w=write_page->width;
    }
    
    //PRE-ERROR CHECKING
    if (passed&1){
        if (row<write_page->top_row) goto error;
        if ((row!=h)&&(row>write_page->bottom_row)){
            if (width8050switch){//note: can assume WIDTH 80,25 as no SCREEN or WIDTH statements have been called
                width8050switch=0;
                if (row<=50){
                    if (passed&2){
                        if (column<1) goto error;
                        if (column>w) goto error;
                    }
                    std::vector<char> buffer(80*25*2);
                    uint32 c,c2;
                    c=write_page->color; c2=write_page->background_color;
                    memcpy(buffer.data(),&cmem[0xB8000],80*25*2);
                    qbsub_width(0,80,50,0,0,3);
                    memcpy(&cmem[0xB8000],buffer.data(),80*25*2);
                    write_page->color=c; write_page->background_color=c2;
                    goto width8050switch_done;
                }
            }
            goto error;
        }
    }
    width8050switch_done:
    if (passed&2){
        if (column<1) goto error;
        if (column>w) goto error;
    }
    if (passed&4){
        if (cursor<0) goto error;
        if (cursor>1) goto error;
    }
    if (passed&8){
        if (start<0) goto error;
        if (start>31) goto error;
    }
    if (passed&16){
        if (stop<0) goto error;
        if (stop>31) goto error;
    }
    
    if (passed&1) {write_page->cursor_y=row; write_page->holding_cursor=0;}
    if (passed&2) {write_page->cursor_x=column; write_page->holding_cursor=0;}
    if ((passed&3)==0){
        if (write_page->holding_cursor) write_page->holding_cursor=2;//special case[forum/index.php?topic=5255.0]
    }
    
    
    if (passed&4){
        if (cursor) cursor=1;
        write_page->cursor_show=cursor;
        if (write_page->flags&IMG_SCREEN){//page-linked attribute
            for (i=0;i<pages;i++){
                if (page[i]) img[i].cursor_show=cursor;
            }
        }//IMG_SCREEN
    }//passed&4
    
    
    if (passed&8){
        write_page->cursor_firstvalue=start;
        }else{
        start=write_page->cursor_firstvalue;
    }
    if (passed&16){
        write_page->cursor_lastvalue=stop;
        }else{
        stop=write_page->cursor_lastvalue;
    }
    if (passed&(8+16)){
        if (write_page->flags&IMG_SCREEN){//page-linked attribute
            for (i=0;i<pages;i++){
                if (page[i]){
                    img[i].cursor_firstvalue=start;
                    img[i].cursor_lastvalue=stop;
                }
            }//i
        }//IMG_SCREEN
    }
    
    return;
    
    error:
    error(5);
    return;
}









//input helper functions:
uint64 hexoct2uint64_value;
int32 hexoct2uint64(qbs *h) {
  // returns 0 = failed
  //         1 = HEX value (default if unspecified)
  //         2 = OCT value
  //         3 = BIN value
  static int32 i, i2;
  static uint64 result;
  result = 0;
  static int32 type;
  type = 0;
  hexoct2uint64_value = 0;
  if (!h->len) return 1;
  if (h->chr[0] != 38) return 0;  // not "&"
  if (h->len == 1) return 1;      // "&" received, but awaiting further input
  i = h->chr[1];
  if ((i == 72) || (i == 104)) type = 1;  // "H" or "h"
  if ((i == 79) || (i == 111)) type = 2;  // "O" or "o"
  if ((i == 66) || (i ==  98)) type = 3;  // "B" or "b"
  if (!type) return 0;
  if (h->len == 2) return type;

  // unsigned _int64 max = 18446744073709551615   (decimal, 20 chars)
  //                     = 1111111 etc., max. 64x (binary, 64 chars)
  //                     = 1777777777777777777777 (octal, 22 chars)
  //                     = FFFFFFFFFFFFFFFF       (hex, 16 chars)

  if (type == 1) {
    if (h->len > 18) return 0;  // larger than int64
    for (i = 2; i < h->len; i++) {
      result <<= 4;
      i2 = h->chr[i];
      //           0      -      9       /       A      -      F       /       a      -      f
      if (((i2 >= 48) && (i2 <= 57)) || ((i2 >= 65) && (i2 <= 70)) || ((i2 >= 97) && (i2 <= 102))) {
        if (i2 >= 97) i2 -= 32;
        if (i2 >= 65) i2 -= 7;
        i2 -= 48;
        // i2 is now a values between 0 and 15
        result += i2;
      } else
        return 0;  // invalid character
    }  // i
    hexoct2uint64_value = result;
    return 1;
  }  // type == 1

  if (type == 2) {
    if (h->len > 24) return 0;  // larger than int64
    if (h->len == 24) {
      if ((h->chr[2] != 48) && (h->chr[2] != 49))
        return 0;  // larger than int64
    }
    for (i = 2; i < h->len; i++) {
      result <<= 3;
      i2 = h->chr[i];
      if ((i2 >= 48) && (i2 <= 55)) {  // 0-7
        i2 -= 48;
        result += i2;
      } else
        return 0;  // invalid character
    }  // i
    hexoct2uint64_value = result;
    return 2;
  }  // type == 2

  if (type == 3) {
    if (h->len > 66) return 0;  // larger than int64
    for (i = 2; i < h->len; i++) {
      result <<= 1;
      i2 = h->chr[i];
      if ((i2 >= 48) && (i2 <= 49)) {  // 0-1
        i2 -= 48;
        result += i2;
      } else
        return 0;  // invalid character
    }  // i
    hexoct2uint64_value = result;
    return 3;
  }  // type == 3
}


//input method (complex, calls other qbs functions)
const char *uint64_max[] =    {"18446744073709551615"};
const char *int64_max[] =     {"9223372036854775807"};
const char *int64_max_neg[] = {"9223372036854775808"};
const char *single_max[] = {"3402823"};
const char *single_max_neg[] = {"1401298"};
const char *double_max[] = {"17976931"};
const char *double_max_neg[] = {"4940656"};
uint8 significant_digits[1024];
int32 num_significant_digits;

extern void *qbs_input_variableoffsets[257];
extern int32 qbs_input_variabletypes[257];
qbs *qbs_input_arguements[257];
int32 cursor_show_last;

void qbs_input(int32 numvariables,uint8 newline){
    if (new_error) return;
    int32 i,i2,i3,i4,i5,i6,chr;
    
    static int32 autodisplay_backup;
    autodisplay_backup=autodisplay;
    autodisplay=1;
    
    static int32 source_backup;
    source_backup=func__source();
    sub__source(func__dest());
    
    //duplicate dest image so any changes can be reverted
    static int32 dest_image,dest_image_temp,dest_image_holding_cursor;
    dest_image=func__copyimage(func__dest(),NULL,NULL);
    if (dest_image==-1) error(516);//out of memory
    dest_image_temp=func__copyimage(func__dest(),NULL,NULL);
    if (dest_image_temp==-1) error(517);//out of memory
    static int32 dest_image_cursor_x,dest_image_cursor_y;
    dest_image_cursor_x=write_page->cursor_x;
    dest_image_cursor_y=write_page->cursor_y;
    dest_image_holding_cursor=write_page->holding_cursor;
    
    uint32 qbs_tmp_base=qbs_tmp_list_nexti;
    
    static int32 lineinput;
    lineinput=0;
    if (qbs_input_variabletypes[1]&ISSTRING){
        if (qbs_input_variabletypes[1]&512){
            qbs_input_variabletypes[1]=-512;
            lineinput=1;
        }}
        
        cursor_show_last=write_page->cursor_show;
        write_page->cursor_show=1;
        
        int32 addspaces;
        addspaces=0;
        qbs* inpstr=qbs_new(0,0);//not temp so must be freed
        qbs* inpstr2=qbs_new(0,0);//not temp so must be freed
        qbs* key=qbs_new(0,0);//not temp so must be freed
        qbs* c=qbs_new(1,0);//not temp so must be freed
        
        for (i=1;i<=numvariables;i++) qbs_input_arguements[i]=qbs_new(0,0);
        
        //init all passed variables to 0 or ""
        for (i=1;i<=numvariables;i++){
            
            if (qbs_input_variabletypes[i]&ISSTRING){//STRING
                if (((qbs*)qbs_input_variableoffsets[i])->fixed){
                    memset(((qbs*)qbs_input_variableoffsets[i])->chr,32,((qbs*)qbs_input_variableoffsets[i])->len);
                    }else{
                    ((qbs*)qbs_input_variableoffsets[i])->len=0;
                }
            }
            
            if ((qbs_input_variabletypes[i]&ISOFFSETINBITS)==0){//reg. numeric variable
                memset(qbs_input_variableoffsets[i],0,(qbs_input_variabletypes[i]&511)>>3);
            }
            
            //bit referenced?
            
        }//i
        
        
        
        
        qbs_input_next:
        
        int32 argn,firstchr,toomany;
        toomany=0;
        argn=1;
        i=0;
        i2=0;
        qbs_input_arguements[1]->len=0;
        firstchr=1;
        qbs_input_sep_arg:
        
        if (i<inpstr->len){
            
            if (inpstr->chr[i]==44){//","
                if (i2!=1){//not in the middle of a string
                    if (!lineinput){
                        i2=0;
                        argn=argn+1;
                        if (argn>numvariables){toomany=1; goto qbs_input_sep_arg_done;}
                        qbs_input_arguements[argn]->len=0;
                        firstchr=1;
                        goto qbs_input_next_arg;
                    }
                }
            }
            
            if (inpstr->chr[i]==34){//"
                if (firstchr){
                    if (!lineinput){
                        i2=1;//requires closure
                        firstchr=0;
                        goto qbs_input_next_arg;
                    }
                }
                if (i2==1){
                    i2=2;
                    goto qbs_input_next_arg;
                }
            }
            
            if (i2==2){
                goto backspace;//INVALID! Cannot have any characters after a closed "..."
            }
            
            c->chr[0]=inpstr->chr[i];
            qbs_set(qbs_input_arguements[argn],qbs_add(qbs_input_arguements[argn],c));
            
            firstchr=0;
            qbs_input_next_arg:;
            i++;
            goto qbs_input_sep_arg;
        }
        qbs_input_sep_arg_done:
        if (toomany) goto backspace;
        
        //validate current arguements
        //ASSUME LEADING & TRALING SPACES REMOVED!
        uint8 valid;
        uint8 neg;
        int32 completewith;
        int32 l;
        uint8 *cp,*cp2;
        uint64 max,max_neg,multiple,value;
        uint64 hexvalue;
        
        completewith=-1;
        valid=1;
        l=qbs_input_arguements[argn]->len;
        cp=qbs_input_arguements[argn]->chr;
        neg=0;
        
        if ((qbs_input_variabletypes[argn]&ISSTRING)==0){
            if ((qbs_input_variabletypes[argn]&ISFLOAT)==0){
                if ((qbs_input_variabletypes[argn]&511)<=32){//cannot handle INTEGER64 variables using this method!
                    int64 finalvalue;
                    //it's an integer variable!
                    finalvalue=0;
                    if (l==0){completewith=48; goto typechecked_integer;}
                    //calculate max & max_neg (i4 used to store number of bits)
                    i4=qbs_input_variabletypes[argn]&511;
                    max=1;
                    max<<=i4;
                    max--;
                    
                    //check for hex/oct/bin
                    if (i3=hexoct2uint64(qbs_input_arguements[argn])){
                        hexvalue=hexoct2uint64_value;
                        if (hexvalue>max){valid=0; goto typechecked;}
                        //i. check max num of "digits" required to represent a value, if more exist cull excess
                        //ii. set completewith value (if necessary)
                        if (i3==1){
                            value=max;
                            i=0;
                            for (i2=1;i2<=16;i2++){
                                if (value&0xF) i=i2;
                                value>>=4;
                            }
                            if (l>(2+i)){valid=0; goto typechecked;}
                            if (l==1) completewith=72;//"H"
                            if (l==2) completewith=48;//"0"
                        }
                        if (i3==2){
                            value=max;
                            i=0;
                            for (i2=1;i2<=22;i2++){
                                if (value&0x7) i=i2;
                                value>>=3;
                            }
                            if (l>(2+i)){valid=0; goto typechecked;}
                            if (l==1) completewith=79;//"O"
                            if (l==2) completewith=48;//"0"
                        }
                        if (i3==3){
                            value=max;
                            i=0;
                            for (i2=1;i2<=64;i2++){
                                if (value&0x1) i=i2;
                                value>>=1;
                            }
                            if (l>(2+i)){valid=0; goto typechecked;}
                            if (l==1) completewith=66;//"B"
                            if (l==2) completewith=48;//"0"
                        }
                        finalvalue=hexvalue;
                        goto typechecked_integer;
                    }
                    
                    //max currently contains the largest UNSIGNED value possible, adjust as necessary
                    if (qbs_input_variabletypes[argn]&ISUNSIGNED){ 
                        max_neg=0;
                        }else{
                        max>>=1;
                        max_neg=max+1;
                    }
                    //check for - sign
                    i2=0;
                    if ((qbs_input_variabletypes[argn]&ISUNSIGNED)==0){ 
                        if (cp[i2]==45){//"-"
                            if (l==1) {completewith=48; goto typechecked_integer;}
                            i2++; neg=1;
                        }
                    }
                    //after a leading 0 no other digits are possible, return an error if this is the case
                    if (cp[i2]==48){
                        if (l>(i2+1)){valid=0; goto typechecked;}
                    }
                    //scan the "number"...
                    multiple=1;
                    value=0;
                    for (i=l-1;i>=i2;i--){
                        i3=cp[i]-48;
                        if ((i3>=0)&&(i3<=9)){
                            value+=multiple*i3;
                            if (qbs_input_variabletypes[argn]&ISUNSIGNED){ 
                                if (value>max){valid=0; goto typechecked;}
                                }else{
                                if (neg){
                                    if (value>max_neg){valid=0; goto typechecked;}
                                    }else{
                                    if (value>max){valid=0; goto typechecked;}
                                }
                            }
                        }else{valid=0; goto typechecked;}
                        multiple*=10;
                    }//next i
                    if (neg) finalvalue=-value; else finalvalue=value;
                    typechecked_integer:
                    //set variable to finalvalue
                    if ((qbs_input_variabletypes[argn]&ISOFFSETINBITS)==0){//reg. numeric variable
                        memcpy(qbs_input_variableoffsets[argn],&finalvalue,(qbs_input_variabletypes[argn]&511)>>3);
                    }
                    goto typechecked;
                }
            }
        }
        
        if (qbs_input_variabletypes[argn]&ISSTRING){
            if (((qbs*)qbs_input_variableoffsets[argn])->fixed){
                if (l>((qbs*)qbs_input_variableoffsets[argn])->len) {valid=0; goto typechecked;}
            }
            qbs_set((qbs*)qbs_input_variableoffsets[argn],qbs_input_arguements[argn]);
            goto typechecked;
        }
        
        //INTEGER64 type
        //int64 range:          \969223372036854775808 to  9223372036854775807
        //uint64 range: 0                    to 18446744073709551615
        if ((qbs_input_variabletypes[argn]&ISSTRING)==0){
            if ((qbs_input_variabletypes[argn]&ISFLOAT)==0){
                if ((qbs_input_variabletypes[argn]&511)==64){
                    if (l==0){completewith=48; *(int64*)qbs_input_variableoffsets[argn]=0; goto typechecked;}
                    
                    //check for hex/oct/bin
                    if (i3=hexoct2uint64(qbs_input_arguements[argn])){
                        hexvalue=hexoct2uint64_value;
                        if (hexvalue>max){valid=0; goto typechecked;}
                        //set completewith value (if necessary)
                        if (i3==1) if (l==1) completewith=72;//"H"
                        if (i3==2) if (l==1) completewith=79;//"O"
                        if (i3==3) if (l==1) completewith=66;//"B"
                        if (l==2) completewith=48;//"0"
                        *(uint64*)qbs_input_variableoffsets[argn]=hexvalue;
                        goto typechecked;
                    }
                    
                    //check for - sign
                    i2=0;
                    if ((qbs_input_variabletypes[argn]&ISUNSIGNED)==0){ 
                        if (cp[i2]==45){//"-"
                            if (l==1) {completewith=48; *(int64*)qbs_input_variableoffsets[argn]=0; goto typechecked;}
                            i2++; neg=1;
                        }
                    }
                    //after a leading 0 no other digits are possible, return an error if this is the case
                    if (cp[i2]==48){
                        if (l>(i2+1)){valid=0; goto typechecked;}
                    }
                    //count how many digits are in the number
                    i4=0;
                    for (i=l-1;i>=i2;i--){
                        i3=cp[i]-48;
                        if ((i3<0)||(i3>9)) {valid=0; goto typechecked;}
                        i4++;
                    }//i
                    if (qbs_input_variabletypes[argn]&ISUNSIGNED){
                        if (i4<20) goto typechecked_int64;
                        if (i4>20) {valid=0; goto typechecked;}
                        
                        cp2=(uint8*)uint64_max[0];
                        }else{
                        if (i4<19) goto typechecked_int64;
                        if (i4>19) {valid=0; goto typechecked;}
                        if (neg) cp2=(uint8*)int64_max_neg[0]; else cp2=(uint8*)int64_max[0];
                    }
                    //number of digits valid, but exact value requires checking
                    cp=qbs_input_arguements[argn]->chr;
                    for (i=0;i<i4;i++){
                        if (cp[i+i2]<cp2[i]) goto typechecked_int64;
                        if (cp[i+i2]>cp2[i]) {valid=0; goto typechecked;}
                    }
                    typechecked_int64:
                    //add character 0 to end to make it a null terminated string
                    c->chr[0]=0; qbs_set(qbs_input_arguements[argn],qbs_add(qbs_input_arguements[argn],c));
                    if (qbs_input_variabletypes[argn]&ISUNSIGNED){
                        #ifdef QBNex_WINDOWS
                            sscanf((char*)qbs_input_arguements[argn]->chr,"%I64u",(uint64*)qbs_input_variableoffsets[argn]);
                            #else
                            sscanf((char*)qbs_input_arguements[argn]->chr,"%llu",(uint64*)qbs_input_variableoffsets[argn]);
                        #endif
                        }else{
                        #ifdef QBNex_WINDOWS
                            sscanf((char*)qbs_input_arguements[argn]->chr,"%I64i",(int64*)qbs_input_variableoffsets[argn]);
                            #else
                            sscanf((char*)qbs_input_arguements[argn]->chr,"%lli",(int64*)qbs_input_variableoffsets[argn]);
                        #endif
                    }
                    goto typechecked;
                }
            }
        }
        
        //check ISFLOAT type?
        //[-]9999[.]9999[E/D][+/-]99999
        if (qbs_input_variabletypes[argn]&ISFLOAT){
            static int32 digits_before_point;
            static int32 digits_after_point;
            static int32 zeros_after_point;
            static int32 neg_power;
            digits_before_point=0;
            digits_after_point=0;
            neg_power=0;
            value=0;
            zeros_after_point=0;
            num_significant_digits=0;
            
            //set variable to 0
            if ((qbs_input_variabletypes[argn]&511)==32) *(float*)qbs_input_variableoffsets[argn]=0;
            if ((qbs_input_variabletypes[argn]&511)==64) *(double*)qbs_input_variableoffsets[argn]=0;
            if ((qbs_input_variabletypes[argn]&511)==256) *(long double*)qbs_input_variableoffsets[argn]=0;
            
            //begin with a generic assessment, regardless of whether it is single, double or float
            if (l==0){completewith=48; goto typechecked;}
            
            //check for hex/oct/bin
            if (i3=hexoct2uint64(qbs_input_arguements[argn])){
                hexvalue=hexoct2uint64_value;
                //set completewith value (if necessary)
                if (i3==1) if (l==1) completewith=72;//"H"
                if (i3==2) if (l==1) completewith=79;//"O"
                if (i3==3) if (l==1) completewith=66;//"B"
                if (l==2) completewith=48;//"0"
                //nb. because VC6 didn't support...
                //error C2520: conversion from uint64 to double not implemented, use signed int64
                //I've implemented a work-around so correct values will be returned
                static int64 transfer;
                transfer=0x7FFFFFFF;
                transfer<<=32;
                transfer|=0xFFFFFFFF;
                while(hexvalue>transfer){
                    hexvalue-=transfer;
                    if ((qbs_input_variabletypes[argn]&511)==32) *(float*)qbs_input_variableoffsets[argn]+=transfer;
                    if ((qbs_input_variabletypes[argn]&511)==64) *(double*)qbs_input_variableoffsets[argn]+=transfer;
                    if ((qbs_input_variabletypes[argn]&511)==256) *(long double*)qbs_input_variableoffsets[argn]+=transfer;
                }
                transfer=hexvalue;
                if ((qbs_input_variabletypes[argn]&511)==32) *(float*)qbs_input_variableoffsets[argn]+=transfer;
                if ((qbs_input_variabletypes[argn]&511)==64) *(double*)qbs_input_variableoffsets[argn]+=transfer;
                if ((qbs_input_variabletypes[argn]&511)==256) *(long double*)qbs_input_variableoffsets[argn]+=transfer;
                goto typechecked;
            }
            
            //check for - sign
            i2=0;
            if (cp[i2]==45){//"-"
                if (l==1) {completewith=48; goto typechecked;}
                i2++; neg=1;
            }
            //if it starts with 0, it may only have one leading 0
            if (cp[i2]==48){
                if (l>(i2+1)){
                    i2++;
                    if (cp[i2]==46) goto decimal_point;
                    valid=0; goto typechecked;//expected a decimal point
                    //nb. of course, user could have typed D or E BUT there is no point
                    //    calculating 0 to the power of anything!
                }else goto typechecked;//validate, as no other data is required
            }
            //scan digits before decimal place
            for (i=i2;i<l;i++){
                i3=cp[i];
                if ((i3==68)||(i3==(68+32))||(i3==69)||(i3==(69+32))){//d,D,e,E?
                    if (i==i2){valid=0; goto typechecked;}//cannot begin with d,D,e,E!
                    i2=i;
                    goto exponent;
                }
                if (i3==46){i2=i; goto decimal_point;}//nb. it can begin with a decimal point!
                i3-=48;
                if ((i3<0)||(i3>9)){valid=0; goto typechecked;}
                digits_before_point++;
                //nb. because leading 0 is handled differently, all digits are significant
                significant_digits[num_significant_digits]=i3+48; num_significant_digits++;
            }
            goto assess_float;
            ////////////////////////////////
            decimal_point:;
            i4=1;
            if (i2==(l-1)) {completewith=48; goto assess_float;}
            i2++;
            for (i=i2;i<l;i++){
                i3=cp[i];
                if ((i3==68)||(i3==(68+32))||(i3==69)||(i3==(69+32))){//d,D,e,E?
                    if (num_significant_digits){
                        if (i==i2){valid=0; goto typechecked;}//cannot begin with d,D,e,E just after a decimal point!
                        i2=i;
                        goto exponent;
                    }
                }
                i3-=48;
                if ((i3<0)||(i3>9)){valid=0; goto typechecked;}
                if (i3) i4=0;
                if (i4) zeros_after_point++;
                digits_after_point++;
                if ((num_significant_digits)||i3){
                    significant_digits[num_significant_digits]=i3+48; num_significant_digits++;
                }
            }//i
            goto assess_float;
            ////////////////////////////////
            exponent:;
            
            //ban d/D for SINGLE precision input
            if ((qbs_input_variabletypes[argn]&511)==32){//SINGLE
                i3=cp[i2];
                if ((i3==68)||(i3==(68+32))){//d/D
                    valid=0; goto typechecked;
                }
            }
            //correct "D" notation for c++ scanf
            i3=cp[i2];
            if ((i3==68)||(i3==(68+32))){//d/D
                cp[i2]=69;//"E"
            }
            if (i2==(l-1)) {completewith=48; goto assess_float;}
            i2++;
            //check for optional + or -
            i3=cp[i2];
            if (i3==45){//"-"
                if (i2==(l-1)) {completewith=48; goto assess_float;}
                neg_power=1;
                i2++;
            }
            if (i3==43){//"+"
                if (i2==(l-1)) {completewith=48; goto assess_float;}
                i2++;
            }
            //nothing valid after a leading 0
            if (cp[i2]==48){//0
                if (l>(i2+1)) {valid=0; goto typechecked;}
            }
            multiple=1;
            value=0;
            for (i=l-1;i>=i2;i--){
                i3=cp[i]-48;
                if ((i3>=0)&&(i3<=9)){
                    
                    value+=multiple*i3;
                    }else{
                    valid=0; goto typechecked;
                }
                multiple*=10;
            }//i
            //////////////////////////
            assess_float:;
            //nb. 0.???? means digits_before_point==0
            
            if ((qbs_input_variabletypes[argn]&511)==32){//SINGLE
                //QB:           \B13.402823    E+38 to \B11.401298    E-45
                //WIKIPEDIA:    \B13.4028234   E+38 to ?
                //OTHER SOURCE: \B13.402823466 E+38 to \B11.175494351 E-38
                if (neg_power) value=-value;
                //special case->single 0 after point
                if ((zeros_after_point==1)&&(digits_after_point==1)){
                    digits_after_point=0;
                    zeros_after_point=0;
                }
                //upper overflow check
                //i. check that value doesn't consist solely of 0's
                if (zeros_after_point>43){valid=0; goto typechecked;}//cannot go any further without reversal by exponent
                if ((digits_before_point==0)&&(digits_after_point==zeros_after_point)) goto nooverflow_float;
                //ii. calculate the position of the first WHOLE digit (in i)
                i=digits_before_point;
                if (!i) i=-zeros_after_point;
                /*EXAMPLES:
                    1.0         i=1
                    12.0        i=2
                    0.1         i=0
                    0.01        i=-1
                */
                i=i+value;//apply exponent
                if (i>39){valid=0; goto typechecked;}
                //nb. the above blocks the ability to type a long-int32 number and use a neg exponent
                //    to validate it
                //********IMPORTANT: if i==39 then the first 7 digits MUST be scanned!!!
                if (i==39){
                    cp2=(uint8*)single_max[0];
                    i2=num_significant_digits;
                    if (i2>7) i2=7;
                    for (i3=0;i3<i2;i3++){
                        if (significant_digits[i3]>*cp2){valid=0; goto typechecked;}
                        if (significant_digits[i3]<*cp2) break;
                        cp2++;
                    }
                }
                //check for pointless levels of precision (eg. 1.21351273512653625116212!)
                if (digits_after_point){
                    if (digits_before_point){
                        if ((digits_after_point+digits_before_point)>8){valid=0; goto typechecked;}
                        }else{
                        if ((digits_after_point-zeros_after_point)>8){valid=0; goto typechecked;}
                    }
                }
                //check for "under-flow"
                if (i<-44){valid=0; goto typechecked;}
                //********IMPORTANT: if i==-44 then the first 7 digits MUST be scanned!!!
                if (i==-44){
                    cp2=(uint8*)single_max_neg[0];
                    i2=num_significant_digits;
                    if (i2>7) i2=7;
                    for (i3=0;i3<i2;i3++){
                        if (significant_digits[i3]<*cp2){valid=0; goto typechecked;}
                        if (significant_digits[i3]>*cp2) break;
                        cp2++;
                    }
                }
                nooverflow_float:;
                c->chr[0]=0; qbs_set(qbs_input_arguements[argn],qbs_add(qbs_input_arguements[argn],c));
                sscanf((char*)qbs_input_arguements[argn]->chr,"%f",(float*)qbs_input_variableoffsets[argn]);
                goto typechecked;
            }
            
            if ((qbs_input_variabletypes[argn]&511)==64){//DOUBLE
                //QB: Double (15-digit) precision \B11.7976931 D+308 to \B14.940656 D-324
                //WIKIPEDIA:    \B11.7976931348623157 D+308 to ???
                //OTHER SOURCE: \B11.7976931348623157 D+308 to \B12.2250738585072014E-308
                
                
                
                if (neg_power) value=-value;
                //special case->single 0 after point
                if ((zeros_after_point==1)&&(digits_after_point==1)){
                    digits_after_point=0;
                    zeros_after_point=0;
                }
                //upper overflow check
                //i. check that value doesn't consist solely of 0's
                if (zeros_after_point>322){valid=0; goto typechecked;}//cannot go any further without reversal by exponent
                if ((digits_before_point==0)&&(digits_after_point==zeros_after_point)) goto nooverflow_double;
                //ii. calculate the position of the first WHOLE digit (in i)
                i=digits_before_point;
                if (!i) i=-zeros_after_point;
                i=i+value;//apply exponent
                if (i>309){valid=0; goto typechecked;}
                //nb. the above blocks the ability to type a long-int32 number and use a neg exponent
                //    to validate it
                //********IMPORTANT: if i==309 then the first 8 digits MUST be scanned!!!
                if (i==309){
                    cp2=(uint8*)double_max[0];
                    i2=num_significant_digits;
                    if (i2>8) i2=8;
                    for (i3=0;i3<i2;i3++){
                        if (significant_digits[i3]>*cp2){valid=0; goto typechecked;}
                        if (significant_digits[i3]<*cp2) break;
                        cp2++;
                    }
                }
                //check for pointless levels of precision (eg. 1.21351273512653625116212!)
                if (digits_after_point){
                    if (digits_before_point){
                        if ((digits_after_point+digits_before_point)>16){valid=0; goto typechecked;}
                        }else{
                        if ((digits_after_point-zeros_after_point)>16){valid=0; goto typechecked;}
                    }
                }
                //check for "under-flow"
                if (i<-323){valid=0; goto typechecked;}
                //********IMPORTANT: if i==-323 then the first 7 digits MUST be scanned!!!
                if (i==-323){
                    cp2=(uint8*)double_max_neg[0];
                    i2=num_significant_digits;
                    if (i2>7) i2=7;
                    for (i3=0;i3<i2;i3++){
                        if (significant_digits[i3]<*cp2){valid=0; goto typechecked;}
                        if (significant_digits[i3]>*cp2) break;
                        cp2++;
                    }
                }
                nooverflow_double:;
                c->chr[0]=0; qbs_set(qbs_input_arguements[argn],qbs_add(qbs_input_arguements[argn],c));
                sscanf((char*)qbs_input_arguements[argn]->chr,"%lf",(double*)qbs_input_variableoffsets[argn]);
                goto typechecked;
            }
            
            if ((qbs_input_variabletypes[argn]&511)==256){//FLOAT
                //at present, there is no defined limit for FLOAT type numbers, so no restrictions
                //are applied!
                c->chr[0]=0; qbs_set(qbs_input_arguements[argn],qbs_add(qbs_input_arguements[argn],c));
                
                //sscanf((char*)qbs_input_arguements[argn]->chr,"%lf",(long double*)qbs_input_variableoffsets[argn]);
                static double sscanf_fix;
                sscanf((char*)qbs_input_arguements[argn]->chr,"%lf",&sscanf_fix);
                *(long double*)qbs_input_variableoffsets[argn]=sscanf_fix;
                
            }
            
            
        }//ISFLOAT
        
        //undefined/uncheckable types fall through as valid!
        typechecked:;
        if (!valid) goto backspace;
        
        
        
        qbs_set(inpstr2,inpstr);
        
        
        //input a key
        
        
        
        qbs_input_invalidinput:
        
        static int32 showing_cursor;
        showing_cursor=0;
        
        qbs_input_wait_for_key:
        
        //toggle box cursor
        if (!write_page->text){
            i=1;
            if ((write_page->font>=32)||(write_page->compatible_mode==256)||(write_page->compatible_mode==32)){
                if (GetTicks()&512) i=0;
            }
            if (i!=showing_cursor){
                showing_cursor^=1;
                static int32 x,y,x2,y2,fx,fy,alpha,cw;
                static uint32 c;
                alpha=write_page->alpha_disabled; write_page->alpha_disabled=1;
                fy=fontheight[write_page->font];
                fx=fontwidth[write_page->font]; if (!fx) fx=1;
                cw=fx; if ((write_page->font>=32)||(write_page->compatible_mode==256)||(write_page->compatible_mode==32)) cw=1;
                y2=(write_page->cursor_y-1)*fy;
                for (y=0;y<fy;y++){
                    x2=(write_page->cursor_x-1)*fx;
                    for (x=0;x<cw;x++){
                        pset (x2,y2,point(x2,y2)^write_page->color);
                        x2++;
                    }
                    y2++;
                }
                write_page->alpha_disabled=alpha;
            }
        }//!write_page->text
        
        if (addspaces){
            addspaces--;
            c->chr[0]=32; qbs_set(key,c);
            }else{
            
            if (write_page->console){
                qbs_set(key,qbs_new_txt(""));
                chr=fgetc(stdin);
                if (chr!=EOF){
                    if (chr=='\n') chr=13;
                    qbs_set(key,qbs_new_txt(" "));  
                    key->chr[0]=chr;  
                }else{Sleep(10);}
                }else{
                Sleep(10);
                qbs_set(key,qbs_inkey());
                
            }
            
            qbs_cleanup(qbs_tmp_base,0);
        }
        if (stop_program) return;
        if (key->len!=1) goto qbs_input_wait_for_key;
        
        //remove box cursor
        if (!write_page->text){
            if (showing_cursor){
                showing_cursor=0;
                static int32 x,y,x2,y2,fx,fy,cw,alpha;
                static uint32 c;
                alpha=write_page->alpha_disabled; write_page->alpha_disabled=1;
                fy=fontheight[write_page->font];
                fx=fontwidth[write_page->font]; if (!fx) fx=1;
                cw=fx; if ((write_page->font>=32)||(write_page->compatible_mode==256)||(write_page->compatible_mode==32)) cw=1;
                y2=(write_page->cursor_y-1)*fy;
                for (y=0;y<fy;y++){
                    x2=(write_page->cursor_x-1)*fx;
                    for (x=0;x<cw;x++){
                        pset (x2,y2,point(x2,y2)^write_page->color);
                        x2++;
                    }
                    y2++;
                }
                write_page->alpha_disabled=alpha;
            }
        }//!write_page->text
        
        //input should disallow certain characters
        if (key->chr[0]==7) {qbs_print(key,0); goto qbs_input_next;}//beep!
        if (key->chr[0]==10) goto qbs_input_next;//linefeed
        if (key->chr[0]==9){//tab
            i=8-(inpstr2->len&7);
            addspaces=i;
            goto qbs_input_next;
        }
        //other ASCII chars that cannot be printed
        if (key->chr[0]==11) goto qbs_input_next;
        if (key->chr[0]==12) goto qbs_input_next;
        if (key->chr[0]==28) goto qbs_input_next;
        if (key->chr[0]==29) goto qbs_input_next;
        if (key->chr[0]==30) goto qbs_input_next;
        if (key->chr[0]==31) goto qbs_input_next;
        
        if (key->chr[0]==13){
            //assume input is valid
            
            //autofinish (if necessary)
            
            //assume all parts entered
            
            for (i=1;i<=numvariables;i++){
                qbs_free(qbs_input_arguements[i]);
            }
            
            if (newline){
                c->len=0;
                if (!write_page->console) qbs_print(c,1);
            }
            qbs_free(c);
            qbs_free(inpstr);
            qbs_free(inpstr2);
            qbs_free(key);
            
            write_page->cursor_show=cursor_show_last;
            
            sub__freeimage(dest_image,1); sub__freeimage(dest_image_temp,1);
            
            if (autodisplay_backup==0){
                autodisplay=-1;//toggle request
                while(autodisplay) Sleep(1);
            }
            
            sub__source(source_backup);
            
            return;
        }
        
        if (key->chr[0]==8){//backspace
            backspace:
            if (!inpstr->len) goto qbs_input_invalidinput;
            inpstr->len--;
            i2=func__dest();//backup current dest
            sub_pcopy(dest_image,dest_image_temp);//copy original background to temp
            //write characters to temp
            sub__dest(dest_image_temp);
            write_page->cursor_x=dest_image_cursor_x; write_page->cursor_y=dest_image_cursor_y; write_page->holding_cursor=dest_image_holding_cursor;
            for (i=0;i<inpstr->len;i++){key->chr[0]=inpstr->chr[i]; qbs_print(key,0);}
            sub__dest(i2);
            //copy temp to dest
            sub_pcopy(dest_image_temp,i2);
            //update cursor
            write_page->cursor_x=img[-dest_image_temp].cursor_x; write_page->cursor_y=img[-dest_image_temp].cursor_y;
            goto qbs_input_next;
        }
        
        //affect inpstr2 with key
        qbs_set(inpstr2,qbs_add(inpstr2,key));
        
        //perform actual update
        if (!write_page->console) qbs_print(key,0);
        
        qbs_set(inpstr,inpstr2);
        
        goto qbs_input_next;
        
}//qbs_input

// VAL lives in the Rust runtime (crates/qb-runtime/src/val.rs).
long double func_val(qbs *s){
    unsigned char result[10];
    int32 code=qb_val(s->chr,s->len,result);
    if (code){error(code); return 0;}
    #if LDBL_MANT_DIG == 64
        long double value=0;
        memcpy(&value,result,10);
        return value;
    #else
        return qb_f80_to_f64(result);
    #endif
}






int32 unsupported_port_accessed=0;

int32 H3C7_palette_register_read_index=0;
int32 H3C8_palette_register_index=0;
int32 H3C9_next=0;
int32 H3C9_read_next=0;

int32 H3C0_blink_enable = 1;

void sub__blink(int32 onoff){
    if (onoff==1) H3C0_blink_enable=1; else H3C0_blink_enable=0;
}

int32 func__blink(){
    return -H3C0_blink_enable;
}

int64 func__handle(){
    #ifdef QBNex_WINDOWS
        #ifdef DEPENDENCY_CONSOLE_ONLY
            if (!window_handle) {
                char pszConsoleTitle[1024];
                GetConsoleTitle(pszConsoleTitle,1024);
                window_handle = FindWindow(NULL, pszConsoleTitle);
            }
            return (ptrszint)window_handle;
        #endif

        if (!screen_hide){
            while (!window_exists){Sleep(100);}
            while (!window_handle){Sleep(100);}
            return (ptrszint)window_handle;
        }
    #endif
    
    return 0;
}

qbs *func__title(){
    if (!window_title){
        return qbs_new_txt("");
        }else{
        return qbs_new_txt((char*)window_title);
    }
}

void set_foreground_window(ptrszint i) {
    #ifdef QBNex_WINDOWS
        BOOL result = SetForegroundWindow((HWND) i);
    #endif
    return;
}

int32 func__hasfocus() {
    #ifdef QBNex_GUI
        #ifdef QBNex_WINDOWS
            while (!window_handle){Sleep(100);}
            return -(window_handle==GetForegroundWindow());
            #elif defined(QBNex_LINUX)
            return window_focused;
        #endif
    #endif
    return -1;
}

void sub_out(int32 port,int32 data){
    if (new_error) return;
    unsupported_port_accessed=0;
    port=port&65535;
    data=data&255;
    
    if (port==0x3C0) {
        H3C0_blink_enable = data&(1<<3);
        goto done;
    }
    
    if (port==0x3C7){//&H3C7, set palette register read index
        H3C7_palette_register_read_index=data;
        H3C9_read_next=0;
        goto done;
    }
    if (port==968){//&H3C8, set palette register write index
        H3C8_palette_register_index=data;
        H3C9_next=0;
        goto done;
    }
    if (port==969){//&H3C9, set palette color
        data=data&63;
        if (write_page->pal){//avoid NULL pointer
            if (H3C9_next==0){//red
                write_page->pal[H3C8_palette_register_index]&=0xFF00FFFF;
                write_page->pal[H3C8_palette_register_index]+=(qbr((double)data*4.063492f-0.4999999f)<<16);
            }
            if (H3C9_next==1){//green
                write_page->pal[H3C8_palette_register_index]&=0xFFFF00FF;
                write_page->pal[H3C8_palette_register_index]+=(qbr((double)data*4.063492f-0.4999999f)<<8);
            }
            if (H3C9_next==2){//blue
                write_page->pal[H3C8_palette_register_index]&=0xFFFFFF00;
                write_page->pal[H3C8_palette_register_index]+=qbr((double)data*4.063492f-0.4999999f);
            }
        }
        H3C9_next=H3C9_next+1;
        if (H3C9_next==3){
            H3C9_next=0;
            H3C8_palette_register_index=H3C8_palette_register_index+1;
            H3C8_palette_register_index&=0xFF;
        }
        goto done;
    }
    
    unsupported_port_accessed=1;
    done:
    return;
    error:
    error(5);
}

// RND / RANDOMIZE live in the Rust runtime (crates/qb-runtime/src/rnd.rs).
void sub_randomize (double seed,int32 passed){
    if (new_error) return;
    if (passed==3||passed==1){qb_randomize(seed,passed==3); return;}
    qbs_print(qbs_new_txt("Random-number seed (-32768 to 32767)? "),0);
    static int16 integerseed;
    qbs_input_variabletypes[1]=16;//id.t=16 'a signed 16 bit integer
    qbs_input_variableoffsets[1]=&integerseed;
    qbs_input(1,1);
    qb_randomize_prompt(integerseed);
}

float func_rnd(float n,int32 passed){
    if (new_error) return 0;
    return qb_rnd(n,passed);
}

// TIMER arithmetic lives in the Rust runtime (crates/qb-runtime/src/timer.rs).
double func_timer(double accuracy,int32 passed){
    if (new_error) return 0;
    double value;
    int32 code=qb_timer(GetTicks(),clock_firsttimervalue,qbnex_firsttimervalue,accuracy,passed,&value);
    if (code){error(code); return 0;}
    return value;
}

//_DELAY, _LIMIT: crates/qb-runtime/src/waits.rs
void sub__delay(double seconds){
    if (new_error) return;
    if (int32 e=qb_delay(seconds)) error(e);
}

void sub__fps(double fps, int32 passed){
    //passed=1 means _AUTO
    //passed=2 means use fps
    if (new_error) return;
    if (passed!=1 && passed!=2){error(5); return;}
    if (passed==1){
        auto_fps=1;//_AUTO
    }
    if (passed==2){
        if (fps<1){error(5); return;}
        if (fps>200) fps=200;
        max_fps=fps;
        auto_fps=0;
    }
}

void sub__limit(double fps){
    if (new_error) return;
    if (int32 e=qb_limit(fps)) error(e);
}




//generic_put/get, OPEN, CLOSE, PRINT #: implemented in Rust (crates/qb-runtime/src/fileio.rs)
int32 generic_put(int32 i,int32 offset,uint8 *cp,int32 bytes){
    //offset is a byte-offset from base 0 (-1=current pos); the return value is always 0
    if (new_error) return 0;
    int32 e=qb_generic_put(i,offset,cp,bytes); if (e) error(e);
    return 0;
}

int32 generic_get_bytes_read;
int32 generic_get(int32 i,int32 offset,uint8 *cp,int32 bytes){
    generic_get_bytes_read=0;
    if (new_error) return 0;
    int32 e=qb_generic_get(i,offset,cp,bytes,&generic_get_bytes_read); if (e) error(e);
    return 0;
}

void sub_open(qbs *name,int32 type,int32 access,int32 sharing,int32 i,int64 record_length,int32 passed){
    if (new_error) return;
    int32 e=qb_file_open(name->chr,name->len,type,access,sharing,i,record_length,passed); if (e) error(e);
}

void sub_open_gwbasic(qbs *typestr,int32 i,qbs *name,int64 record_length,int32 passed){
    if (new_error) return;
    int32 e=qb_file_open_gwbasic(typestr->chr,typestr->len,i,name->chr,name->len,record_length,passed); if (e) error(e);
}

void sub_close(int32 i2,int32 passed){
    if (new_error) return;
    int32 i,x;//<--RECURSIVE function - do not make this static

    if (passed){

        if (i2<0){//special handle
            //determine which close procedure to call
            x=-(i2+1);
            static special_handle_struct *sh; sh=(special_handle_struct*)list_get(special_handles,x); if (!sh) return;
            if (sh->type==1){//stream
                static stream_struct *st; st=(stream_struct*)sh->index;
                if (st->type==1){//connection
                    connection_close(x);
                }//connection
            }//stream

            if (sh->type==2){//host listener
                connection_close(x);
            }//host listener

            return;
        }//special handle

        qb_file_close(i2);
        return;

    }//passed

    //special handles
    for (i=1;i<=special_handles->indexes;i++){
        if (vwatch>0 && vwatch==i) {
            //keep the debug connection open for $DEBUG mode
        } else {
            sub_close(-i-1,1);
        }
    }

    qb_file_close_all();

}//close

void sub_file_print(int32 i,qbs *str,int32 extraspace,int32 tab,int32 newline){
    if (new_error) return;
    //tcp/ip?
    //note: spacing considerations such as 'extraspace' & 'tab' are ignored
    if (i<0){
        return;
    }
    int32 e=qb_file_print(i,str->chr,str->len,extraspace,tab,newline);
    if (e==QB_PRINT_TO_SCREEN){
        qbs_print (str, newline);
        return;
    }
    if (e) error(e);
}

//READ/DATA and the universal number representation (n_float, n_int64, n_uint64):
//crates/qb-runtime/src/numparse.rs

//INPUT # readers: implemented in Rust (crates/qb-runtime/src/fileio.rs). The
//number reader fills the universal number representation used by n_float.
static qb_num_repr n_input;//number read by INPUT #
int32 n_inputnumberfromfile(int32 fileno){
    //return values: 0=success, 1=overflow, 2=eof, 3=failed (no further errors)
    int32 r=3;
    int32 e=qb_file_input_number(fileno,&n_input,&r);
    if (e) error(e);
    return r;
}

static long double num_value(const qb_num_value &v){
    return v.is_int?(long double)v.i:(long double)v.f;
}


void revert_input_check(){
}

void sub_file_input_string(int32 fileno,qbs *deststr){
    if (new_error) return;
    //tcp/ip?
    //note: spacing considerations are ignored
    if (fileno<0){
        return;
    }
    const uint8 *out; int32 len=0,assign=0;
    int32 e=qb_file_input_string(fileno,&out,&len,&assign);
    if (assign) qbs_set(deststr,qbs_new_txt_len((const char*)out,len));
    if (e) error(e);
}

int64 func_file_input_int64(int32 fileno){
    if (new_error) return 0;
    static int32 i;
    i=n_inputnumberfromfile(fileno);
    if (i==1){error(6); return 0;}//overflow
    if (i==2){error(62); return 0;}//input past end of file
    if (i==3) return 0;//failed
    int64 v; if (qb_num_to_int64(&n_input,&v)) return v;
    error(6);//overflow
    return 0;
}

uint64 func_file_input_uint64(int32 fileno){
    if (new_error) return 0;
    static int32 i;
    i=n_inputnumberfromfile(fileno);
    if (i==1){error(6); return 0;}//overflow
    if (i==2){error(62); return 0;}//input past end of file
    if (i==3) return 0;//failed
    uint64 v; if (qb_num_to_uint64(&n_input,&v)) return v;
    error(6);//overflow
    return 0;
}




void sub_read_string(uint8 *data,ptrszint *data_offset,ptrszint data_size,qbs *deststr){
    if (new_error) return;
    const uint8 *out; int32 len=0;
    int32 e=qb_read_string(data,data_size,data_offset,&out,&len);
    if (e) error(e);
    qbs_set(deststr,qbs_new_txt_len((const char*)out,len));
}

long double func_read_float(uint8 *data,ptrszint *data_offset,ptrszint data_size,int32 typ){
    if (new_error) return 0;
    qb_num_value v;
    if (int32 e=qb_read_float(data,data_size,data_offset,typ,&v)){error(e); return 0;}
    return num_value(v);
}

int64 func_read_int64(uint8 *data,ptrszint *data_offset,ptrszint data_size){
    if (new_error) return 0;
    int64 v;
    if (int32 e=qb_read_int64(data,data_size,data_offset,&v)){error(e); return 0;}
    return v;
}

uint64 func_read_uint64(uint8 *data,ptrszint *data_offset,ptrszint data_size){
    if (new_error) return 0;
    uint64 v;
    if (int32 e=qb_read_uint64(data,data_size,data_offset,&v)){error(e); return 0;}
    return v;
}

long double func_file_input_float(int32 fileno,int32 typ){
    if (new_error) return 0;
    static int32 i;
    i=n_inputnumberfromfile(fileno);
    if (i==1){error(6); return 0;}//overflow
    if (i==2){error(62); return 0;}//input past end of file
    if (i==3) return 0;//failed
    qb_num_value v;
    if (int32 e=qb_num_to_float(&n_input,typ,&v)){error(e); return 0;}
    return num_value(v);
}//func_file_input_float

void *byte_element(uint64 offset,int32 length){
    if (length<0) length=0;//some calculations may result in negative values which mean 0 (no bytes available)
    //add structure to xms stack (byte_element structures are never stored in cmem!)
    void *p;
    if ((mem_static_pointer+=12)<mem_static_limit) p=mem_static_pointer-12; else p=mem_static_malloc(12);
    *((uint64*)p)=offset;
    ((uint32*)p)[2]=length;
    return p;
}

void *byte_element(uint64 offset,int32 length,byte_element_struct *info){
    if (length<0) length=0;//some calculations may result in negative values which mean 0 (no bytes available)
    info->length=length;
    info->offset=offset;
    return info;
}

void call_interrupt(int32 intno, void *inregs,void *outregs){
    if (new_error) return;
    static byte_element_struct *ele;
    static uint16 *sp;
    /* testing only
        static qbs* s=NULL;
        if (s==NULL) s=qbs_new(0,0);
        qbs_set(s,qbs_str(ele->length));
        MessageBox2(NULL,(char*)s->chr,"CALL INTERRUPT: size",MB_OK|MB_SYSTEMMODAL);
        qbs_set(s,qbs_str( ((uint8*)(ele->offset))[0] ));
        MessageBox2(NULL,(char*)s->chr,"CALL INTERRUPT: value",MB_OK|MB_SYSTEMMODAL);
    */
    /* reference
        TYPE RegType
        AX AS INTEGER
        BX AS INTEGER
        CX AS INTEGER
        DX AS INTEGER
        BP AS INTEGER
        SI AS INTEGER
        DI AS INTEGER
        FLAGS AS INTEGER
        END TYPE
    */
    //error checking
    ele=(byte_element_struct*)outregs;
    if (ele->length<16){error(5); return;}//Illegal function call
    ele=(byte_element_struct*)inregs;
    if (ele->length<16){error(5); return;}//Illegal function call
    //load virtual registers
    sp=(uint16*)(ele->offset);
    cpu.ax=sp[0];
    cpu.bx=sp[1];
    cpu.cx=sp[2];
    cpu.dx=sp[3];
    cpu.bp=sp[4];
    cpu.si=sp[5];
    cpu.di=sp[6];
    //note: flags ignored (revise)
    call_int(intno);
    //save virtual registers
    ele=(byte_element_struct*)outregs;
    sp=(uint16*)(ele->offset);
    sp[0]=cpu.ax;
    sp[1]=cpu.bx;
    sp[2]=cpu.cx;
    sp[3]=cpu.dx;
    sp[4]=cpu.bp;
    sp[5]=cpu.si;
    sp[6]=cpu.di;
    //note: flags ignored (revise)
    return;
}

void call_interruptx(int32 intno, void *inregs,void *outregs){
    if (new_error) return;
    static byte_element_struct *ele;
    static uint16 *sp;
    /* reference
        TYPE RegTypeX
        AX AS INTEGER
        BX AS INTEGER
        CX AS INTEGER
        DX AS INTEGER
        BP AS INTEGER
        SI AS INTEGER
        DI AS INTEGER
        FLAGS AS INTEGER
        DS AS INTEGER
        ES AS INTEGER
        END TYPE
    */
    //error checking
    ele=(byte_element_struct*)outregs;
    if (ele->length<20){error(5); return;}//Illegal function call
    ele=(byte_element_struct*)inregs;
    if (ele->length<20){error(5); return;}//Illegal function call
    //load virtual registers
    sp=(uint16*)(ele->offset);
    cpu.ax=sp[0];
    cpu.bx=sp[1];
    cpu.cx=sp[2];
    cpu.dx=sp[3];
    cpu.bp=sp[4];
    cpu.si=sp[5];
    cpu.di=sp[6];
    //note: flags ignored (revise)
    cpu.ds=sp[8];
    cpu.es=sp[9];
    call_int(intno);
    //save virtual registers
    ele=(byte_element_struct*)outregs;
    sp=(uint16*)(ele->offset);
    sp[0]=cpu.ax;
    sp[1]=cpu.bx;
    sp[2]=cpu.cx;
    sp[3]=cpu.dx;
    sp[4]=cpu.bp;
    sp[5]=cpu.si;
    sp[6]=cpu.di;
    //note: flags ignored (revise)
    sp[8]=cpu.ds;
    sp[9]=cpu.es;
    return;
}

//GET / PUT: file numbers are handled in Rust (crates/qb-runtime/src/fileio.rs);
//TCP/IP streams (negative handles) stay here.
void sub_get(int32 i,int64 offset,void *element,int32 passed){
    if (new_error) return;
    static byte_element_struct *ele;
    static int32 x,x2;

    if (i<0){//special handle?
        x=-(i+1);
        static special_handle_struct *sh; sh=(special_handle_struct*)list_get(special_handles,x); if (!sh){error(52); return;}
        if (sh->type==1){//stream
            static stream_struct *st; st=(stream_struct*)sh->index;
            stream_update(st);
            ele=(byte_element_struct*)element;
            if (st->in_size<ele->length){st->eof=1; return;}
            st->eof=0;
            memcpy((void*)(ele->offset),st->in,ele->length);
            x2=st->in_size-ele->length; if (x2) memmove(st->in,st->in+ele->length,x2);
            st->in_size-=ele->length;
            return;
        }//stream
        error(52); return;
    }//special handle

    ele=(byte_element_struct*)element;
    int32 e=qb_file_get(i,offset,(uint8*)ele->offset,ele->length,passed); if (e) error(e);
}

void sub_get2(int32 i,int64 offset,qbs *str,int32 passed){
    if (new_error) return;
    static int32 x;

    if (i<0){//special handle?
        if (str->fixed){//following method is only for variable length strings
            static byte_element_struct tbyte_element_struct;
            sub_get(i,offset,byte_element((uint64)str->chr,str->len,&tbyte_element_struct),passed);
            return;
        }
        x=-(i+1);
        static special_handle_struct *sh; sh=(special_handle_struct*)list_get(special_handles,x); if (!sh){error(52); return;}
        if (sh->type==1){//stream
            static stream_struct *st; st=(stream_struct*)sh->index;
            stream_update(st);
            static qbs* tqbs;
            tqbs=qbs_new(st->in_size,1);
            if (st->in_size) memcpy(tqbs->chr,st->in,st->in_size);
            st->in_size=0;
            st->eof=0;
            qbs_set(str,tqbs);
            return;
        }//stream
        error(52); return;
    }//special handle

    const uint8 *out; int32 len=0,assign=0;
    int32 e=qb_file_get_string(i,offset,str->chr,str->len,passed,&out,&len,&assign);
    if (assign) qbs_set(str,qbs_new_txt_len((const char*)out,len));
    if (e) error(e);
}

void sub_put(int32 i,int64 offset,void *element,int32 passed){
    if (new_error) return;
    static byte_element_struct *ele;
    static int32 x;

    if (i<0){//special handle?
        x=-(i+1);
        static special_handle_struct *sh; sh=(special_handle_struct*)list_get(special_handles,x); if (!sh){error(52); return;}
        if (sh->type==1){//stream
            static stream_struct *st; st=(stream_struct*)sh->index;
            ele=(byte_element_struct*)element;
            stream_out(st,(void*)ele->offset,ele->length);
            return;
        }//stream
        error(52); return;
    }//special handle

    ele=(byte_element_struct*)element;
    int32 e=qb_file_put(i,offset,(const uint8*)ele->offset,ele->length,passed); if (e) error(e);
}

//put2 adds a 2-4 byte length descriptor to the data
//(used to PUT variable length strings in RANDOM mode)
void sub_put2(int32 i,int64 offset,void *element,int32 passed){
    if (new_error) return;
    if (i<0){//special handle?
        sub_put(i,offset,element,passed);//(use standard put call)
        return;
    }
    static byte_element_struct *ele;
    ele=(byte_element_struct*)element;
    int32 e=qb_file_put_string(i,offset,(const uint8*)ele->offset,ele->length,passed); if (e) error(e);
}
    
    
    
    
    
    
    void sub_graphics_get(float x1f,float y1f,float x2f,float y2f,void *element,uint32 mask,int32 passed){
        if (new_error) return;
        byte_element_struct *ele=(byte_element_struct*)element;
        if (int32 e=qb_gfx_get(read_page,x1f,y1f,x2f,y2f,(uint8*)(ele->offset),ele->length,mask,passed)) error(e);
    }
    
    
    
    void sub_graphics_put(float x1f,float y1f,void *element,int32 option,uint32 mask,int32 passed){
        if (new_error) return;
        byte_element_struct *ele=(byte_element_struct*)element;
        if (int32 e=qb_gfx_put(write_page,x1f,y1f,(const uint8*)(ele->offset),ele->length,option,mask,passed)) error(e);
    }
    
    
    
    void sub_date(qbs* date){
        if (new_error) return;
        return;//stub
    }
    
    qbs *func_date(){
        //mm-dd-yyyy
        //0123456789
        static time_t qbnex_tm_val;
        static tm *qbnex_tm;
        //struct tm {
        //        int tm_sec;     /* seconds after the minute - [0,59] */
        //        int tm_min;     /* minutes after the hour - [0,59] */
        //        int tm_hour;    /* hours since midnight - [0,23] */
        //        int tm_mday;    /* day of the month - [1,31] */
        //        int tm_mon;     /* months since January - [0,11] */
        //        int tm_year;    /* years since 1900 */
        //        int tm_wday;    /* days since Sunday - [0,6] */
        //        int tm_yday;    /* days since January 1 - [0,365] */
        //        int tm_isdst;   /* daylight savings time flag */
        //        };
        static int32 x,x2,i;
        static qbs *str;
        str=qbs_new(10,1);
        str->chr[2]=45; str->chr[5]=45;//-
        time(&qbnex_tm_val); if (qbnex_tm_val==-1){error(5); str->len=0; return str;}
        qbnex_tm=localtime(&qbnex_tm_val); if (qbnex_tm==NULL){error(5); str->len=0; return str;}
        x=qbnex_tm->tm_mon; x++; 
        i=0; str->chr[i]=x/10+48; str->chr[i+1]=x%10+48;
        x=qbnex_tm->tm_mday;
        i=3; str->chr[i]=x/10+48; str->chr[i+1]=x%10+48;
        x=qbnex_tm->tm_year; x+=1900;
        i=6;
        x2=x/1000; x=x-x2*1000; str->chr[i]=x2+48; i++;
        x2=x/100; x=x-x2*100; str->chr[i]=x2+48; i++;
        x2=x/10; x=x-x2*10; str->chr[i]=x2+48; i++;
        str->chr[i]=x+48;
        return str;
    }
    
    
    
    
    
    
    
    
    void sub_time(qbs* str){
        if (new_error) return;
        return;//stub
    }
    
    qbs *func_time(){
        //23:59:59 (hh:mm:ss)
        //01234567
        static time_t qbnex_tm_val;
        static tm *qbnex_tm;
        //struct tm {
        //        int tm_sec;     /* seconds after the minute - [0,59] */
        //        int tm_min;     /* minutes after the hour - [0,59] */
        //        int tm_hour;    /* hours since midnight - [0,23] */
        //        int tm_mday;    /* day of the month - [1,31] */
        //        int tm_mon;     /* months since January - [0,11] */
        //        int tm_year;    /* years since 1900 */
        //        int tm_wday;    /* days since Sunday - [0,6] */
        //        int tm_yday;    /* days since January 1 - [0,365] */
        //        int tm_isdst;   /* daylight savings time flag */
        //        };
        static int32 x,x2,i;
        static qbs *str;
        str=qbs_new(8,1);
        str->chr[2]=58; str->chr[5]=58;//:
        time(&qbnex_tm_val); if (qbnex_tm_val==-1){error(5); str->len=0; return str;}
        qbnex_tm=localtime(&qbnex_tm_val); if (qbnex_tm==NULL){error(5); str->len=0; return str;}
        x=qbnex_tm->tm_hour;
        i=0; str->chr[i]=x/10+48; str->chr[i+1]=x%10+48;
        x=qbnex_tm->tm_min;
        i=3; str->chr[i]=x/10+48; str->chr[i+1]=x%10+48;
        x=qbnex_tm->tm_sec;
        i=6; str->chr[i]=x/10+48; str->chr[i+1]=x%10+48;
        return str;
    }
    
    
    int32 func_csrlin(){
        #ifdef QBNex_WINDOWS //Windows console CSRLIN support
            if (write_page->console){
                CONSOLE_SCREEN_BUFFER_INFO cl_bufinfo;
                SECURITY_ATTRIBUTES SecAttribs = {sizeof(SECURITY_ATTRIBUTES), 0, 1};
                HANDLE cl_conout = CreateFileA("CONOUT$", GENERIC_READ | GENERIC_WRITE, FILE_SHARE_READ | FILE_SHARE_WRITE, & SecAttribs, OPEN_EXISTING, 0, 0);
                GetConsoleScreenBufferInfo(cl_conout, & cl_bufinfo);
                return cl_bufinfo.dwCursorPosition.Y + 1;
            }
        #endif
        if (write_page->holding_cursor){
            if (write_page->cursor_y>=write_page->bottom_row) return write_page->bottom_row; else return write_page->cursor_y+1;
        }
        return write_page->cursor_y;
    }
    int32 func_pos(int32 ignore){
        #ifdef QBNex_WINDOWS
            if (write_page->console){ //qbnex console CSRLIN 
                CONSOLE_SCREEN_BUFFER_INFO cl_bufinfo;
                SECURITY_ATTRIBUTES SecAttribs = {sizeof(SECURITY_ATTRIBUTES), 0, 1};
                HANDLE cl_conout = CreateFileA("CONOUT$", GENERIC_READ | GENERIC_WRITE, FILE_SHARE_READ | FILE_SHARE_WRITE, & SecAttribs, OPEN_EXISTING, 0, 0);
                GetConsoleScreenBufferInfo(cl_conout, & cl_bufinfo);
                return cl_bufinfo.dwCursorPosition.X + 1;
            }
        #endif
        if (write_page->holding_cursor) return 1;
        return write_page->cursor_x;
    }
    
    
    
    //LOG, FIX, EXP (SINGLE/DOUBLE), SQR: crates/qb-runtime/src/mathfn.rs
    double func_log(double value){double r; if (int32 e=qb_log(value,&r)) error(e); return r;}
    double func_fix_double(double value){return qb_fix_double(value);}
    long double func_fix_float(long double value){
        if (value<0) return ceil(value); else return floor(value);
    }
    
    //EXP
    double func_exp_single(double value){double r; if (int32 e=qb_exp_single(value,&r)) error(e); return r;}
    long double func_exp_float(long double value){
        if (value<=709.782712893){
            return exp(value);
        }
        error(6); return 0;
    }
    
    
    
    extern int32 sleep_break;//defined in Rust (crates/qb-runtime/src/program.rs)
    
    void sub_sleep(int32 seconds,int32 passed){
        if (new_error) return;

        sleep_break=0;
        double prev,ms,now,elapsed;//cannot be static
        prev=GetTicks();
        ms=1000.0*(double)seconds;
        
        #ifdef QBNex_WINDOWS

            if (read_page->console){ 
                int32 junk=0,junk2=0;
                DWORD dwRet;
                HANDLE hStdin = GetStdHandle (STD_INPUT_HANDLE);
                FlushConsoleInputBuffer(hStdin);
                if (passed){
                            do{
                                now=GetTicks();
                                if (now<prev)return;//value looped?
                                elapsed=now-prev;//elapsed time since prev
                                ms = ms-elapsed;
                                prev=now;
                                dwRet = WaitForSingleObject(hStdin,ms); //this should provide our pause
                                if (dwRet==WAIT_TIMEOUT)return; //and if we timeout without any input, we exit early.
                                if (dwRet==WAIT_OBJECT_0){//this says the console had input
                                    junk=func__getconsoleinput();
                                    if(junk==1){//this is a valid keyboard event.  Let's exit SLEEP in the console.
                                        Sleep(100);//Give the user time to remove their finger from the key, before clearing the buffer.
                                        FlushConsoleInputBuffer(hStdin);//flush the keyboard buffer after, so we don't leave stray events to be processed (such as key up events).
                                        return;
                                    }else{//we had an input event such as the mouse.  Ignore it and clear the buffer so we don't keep responding to mouse inputs
                                        FlushConsoleInputBuffer(hStdin);
                                    }
                                }
                            }while(ms>0);//as long as our timer hasn't expired, we continue to run the loop and countdown the time remaining
                            return; //if we get here, something odd happened.  We should expire automatically with the WAIT_TIMEOUT event before this occurs.
                }
                do{ //ignore all console input unless it's a keydown event
                    junk=func__getconsoleinput();
                }while(junk!=1); //only when junk = 1 do we have a keyboard event
                Sleep(100); //Give the user time to remove their finger from the key, before clearing the buffer.
                FlushConsoleInputBuffer(hStdin); //and flush the keyboard buffer after, so we don't leave stray events to be processed.
                return;
            }
        #endif


        qb_sleep_wait(prev,ms);//crates/qb-runtime/src/waits.rs
    }
    
    //HEX$, OCT$, _BIN$ digits: crates/qb-runtime/src/mathfn.rs
    static qbs *radix_string(int64 value,int32 neg_bits,int32 radix){
        uint8 buf[64];
        int32 n=qb_radix_string(value,neg_bits,radix,buf);
        qbs *str=qbs_new(n,1);
        memcpy(str->chr,buf,n);
        return str;
    }
    qbs *func__bin(int64 value,int32 neg_bits){return radix_string(value,neg_bits,2);}
    
    //note: QBASIC doesn't have a BIN$ function
    //      QBNex   uses 32 bin digits for SINGLE/DOUBLE/FLOAT but if this range is exceeded
    //      it uses up to 64 bin digits before generating an "OVERFLOW" error
    //performs overflow check before calling func__bin
    qbs *func__bin_float(long double value){
        static qbs *str;
        static int64 ivalue;
        static int64 uivalue;
        //ref: uint64 0-18446744073709551615
        //      int64 \969223372036854775808 to 9223372036854775807
        if ((value>=9.223372036854776E18)||(value<=-9.223372036854776E18)){
            //note: ideally, the following line would be used, however, qbr_longdouble_to_uint64 just does the same as qbr
            //if ((value>=1.844674407370956E19)||(value<=-9.223372036854776E18)){
            str=qbs_new(0,1); error(6);//Overflow
            return str;
        }
        if (value>=0){
            uivalue=qbr_longdouble_to_uint64(value);
            ivalue=uivalue;
            }else{
            ivalue=qbr(value);
        }
        return func__bin(ivalue,32);
    }
    
    qbs *func_oct(int64 value,int32 neg_bits){return radix_string(value,neg_bits,8);}
    
    //note: QBASIC uses 11 oct digits for SINGLE/DOUBLE or generates "OVERFLOW" if this range is exceeded
    //      QBNex   uses 11 oct digits for SINGLE/DOUBLE/FLOAT but if this range is exceeded
    //      it uses up to 22 oct digits before generating an "OVERFLOW" error
    //performs overflow check before calling func_oct
    qbs *func_oct_float(long double value){
        static qbs *str;
        static int64 ivalue;
        static int64 uivalue;
        //ref: uint64 0-18446744073709551615
        //      int64 \969223372036854775808 to 9223372036854775807
        if ((value>=9.223372036854776E18)||(value<=-9.223372036854776E18)){
            //note: ideally, the following line would be used, however, qbr_longdouble_to_uint64 just does the same as qbr
            //if ((value>=1.844674407370956E19)||(value<=-9.223372036854776E18)){
            str=qbs_new(0,1); error(6);//Overflow
            return str;
        }
        if (value>=0){
            uivalue=qbr_longdouble_to_uint64(value);
            ivalue=uivalue;
            }else{
            ivalue=qbr(value);
        }
        return func_oct(ivalue,32);
    }
    
    qbs *func_hex(int64 value,int32 neg_size){return radix_string(value,neg_size,16);}
    
    //note: QBASIC uses 8 hex digits for SINGLE/DOUBLE or generates "OVERFLOW" if this range is exceeded
    //      QBNex   uses 8 hex digits for SINGLE/DOUBLE/FLOAT but if this range is exceeded
    //      it uses up to 16 hex digits before generating an "OVERFLOW" error
    //performs overflow check before calling func_hex
    qbs *func_hex_float(long double value){
        static qbs *str;
        static int64 ivalue;
        static int64 uivalue;
        //ref: uint64 0-18446744073709551615
        //      int64 \969223372036854775808 to 9223372036854775807
        if ((value>=9.223372036854776E18)||(value<=-9.223372036854776E18)){
            //note: ideally, the following line would be used, however, qbr_longdouble_to_uint64 just does the same as qbr
            //if ((value>=1.844674407370956E19)||(value<=-9.223372036854776E18)){
            str=qbs_new(0,1); error(6);//Overflow
            return str;
        }
        if (value>=0){
            uivalue=qbr_longdouble_to_uint64(value);
            ivalue=uivalue;
            }else{
            ivalue=qbr(value);
        }
        return func_hex(ivalue,8);
    }
    
    //LBOUND/UBOUND: crates/qb-runtime/src/arrays.rs
    ptrszint func_lbound(ptrszint *array,int32 index,int32 num_indexes){
        ptrszint v=0; if (int32 e=qb_array_bound(array,index,num_indexes,0,&v)){error(e); return 0;}
        return v;
    }
    ptrszint func_ubound(ptrszint *array,int32 index,int32 num_indexes){
        ptrszint v=0; if (int32 e=qb_array_bound(array,index,num_indexes,1,&v)){error(e); return 0;}
        return v;
    }
    
    uint8 port60h_event[256];
    int32 port60h_events=0;
    
    
    int32 func_inp(int32 port){
        static int32 value;
        unsupported_port_accessed=0;
        if ((port>65535)||(port<-65536)){
            error(6); return 0;//Overflow
        }
        port&=0xFFFF;
        
        if (port==0x3C9){//read palette
            if (write_page->pal){//avoid NULL pointer
                //convert 0-255 value to 0-63 value
                if (H3C9_read_next==0){//red
                    value=qbr_double_to_long((((double)((write_page->pal[H3C7_palette_register_read_index]>>16)&255))/3.984376-0.4999999f));
                }
                if (H3C9_read_next==1){//green
                    value=qbr_double_to_long((((double)((write_page->pal[H3C7_palette_register_read_index]>>8)&255))/3.984376-0.4999999f));
                }
                if (H3C9_read_next==2){//blue
                    value=qbr_double_to_long((((double)(write_page->pal[H3C7_palette_register_read_index]&255))/3.984376-0.4999999f));
                }
                H3C9_read_next=H3C9_read_next+1;
                if (H3C9_read_next==3){
                    H3C9_read_next=0;
                    H3C7_palette_register_read_index=H3C7_palette_register_read_index+1;
                    H3C7_palette_register_read_index&=0xFF;
                }
                return value;
            }//->pal
            return 0;//non-palette modes
        }
        
        /*
            3dAh (R):  Input Status #1 Register
            bit   0  Either Vertical or Horizontal Retrace active if set
            1  Light Pen has triggered if set
            2  Light Pen switch is open if set
            3  Vertical Retrace in progress if set
            4-5  Shows two of the 6 color outputs, depending on 3C0h index 12h.
            Attr: Bit 4-5:   Out bit 4  Out bit 5
            0          Blue       Red
            1        I Blue       Green
            2        I Red      I Green
        */
        if (port==0x3DA){
            value=0;
            if (vertical_retrace_happened||vertical_retrace_in_progress){
                vertical_retrace_happened=0;
                value|=8;
            }
            return value;
        }
        
        if (port==0x60){
            //return last scancode event
            if (port60h_events){
                value=port60h_event[0];
                if (port60h_events>1) memmove(port60h_event,port60h_event+1,255);
                port60h_events--;
                return value;
                }else{
                return port60h_event[0];
            }
            
        }
        
        
        
        unsupported_port_accessed=1;
        return 0;//unknown port!
    }
    
    void sub_wait(int32 port,int32 andexpression,int32 xorexpression,int32 passed){
        if (new_error) return;
        //1. read value from port
        //2. value^=xorexpression (if passed!)
        //3. value^=andexpression
        //IMPORTANT: Wait returns immediately if given port is unsupported by QBNex so program
        //           can continue
        static int32 value;
        
        //error & range checking
        if ((port>65535)||(port<-65536)){
            error(6); return;//Overflow
        }
        port&=0xFFFF;
        if ((andexpression<-32768)||(andexpression>65535)){
            error(6); return;//Overflow
        }
        andexpression&=0xFF;
        if (passed){
            if ((xorexpression<-32768)||(xorexpression>65535)){
                error(6); return;//Overflow
            }
        }
        xorexpression&=0xFF;
        
        wait:
        value=func_inp(port);
        if (passed) value^=xorexpression;
        value&=andexpression;
        if (value||unsupported_port_accessed||stop_program) return;
        Sleep(1);
        goto wait;
    }
    
    extern int32 tab_LPRINT;//1=dest is LPRINT image
    extern int32 tab_spc_cr_size; //=1;//default
    extern int32 tab_fileno;
    qbs *func_tab(int32 pos){
        if (new_error) return qbs_new(0,1);
        
        static int32 tab_LPRINT_olddest;
        if (tab_LPRINT){
            if (!lprint_image) qbs_lprint(qbs_new(0,1),0);//send dummy data to init the LPRINT image
            tab_LPRINT_olddest=func__dest();
            sub__dest(lprint_image);
        }
        
        //returns a string to advance to the horizontal position "pos" on either
        //the current line or the next line.
        static int32 w,div,cursor;
  
        static int32 cr_size;         // a local in case file is SCRN
        cr_size = tab_spc_cr_size;    // init to caller's value
        //calculate width in spaces & current position
        if (cr_size==2){
            //print to file
            div=1;
            w=2147483647;
            cursor=1;
            //validate file
            static int32 i;
            i=tab_fileno;
            if (i<0) goto invalid_file;//TCP/IP unsupported
            if (gfs_fileno_valid(i)!=1) goto invalid_file;//Bad file name or number
            i=gfs_fileno[i];//convert fileno to gfs index
            if (gfs_file[i].scrn == 1) {    // going to screen, change the cr size
                cr_size = 1;
            } else {
            cursor=gfs_file[i].column;
            }
            invalid_file:;
        }
        if (cr_size == 1) {
            //print to surface
            if (write_page->text){
                w=write_page->width;
                div=1;
                }else{
                if (fontwidth[write_page->font]){
                    w=write_page->width/fontwidth[write_page->font];
                    div=1;
                    }else{
                    //w=func__printwidth(singlespace,NULL,0);
                    w=write_page->width;
                    div=func__printwidth(singlespace,NULL,0);
                }
            }
            cursor=write_page->cursor_x;
        }
        
        static qbs *tqbs;
        if ((pos<-32768)||(pos>32767)){
            if (tab_LPRINT) sub__dest(tab_LPRINT_olddest);
            tqbs=qbs_new(0,1);
            error(6); return tqbs;//Overflow
        }
        if (pos>w) pos%=w;
        if (pos<1) pos=1;
        static int32 size,spaces,cr;
        size=0; spaces=0; cr=0;
        if (cursor>pos){
            cr=1;
            size=cr_size;
            spaces=pos/div; if (pos%div) spaces++;
            spaces--;//don't put a space on the dest position
            size+=spaces;
            }else{
            spaces=(pos-cursor)/div; if ((pos-cursor)%div) spaces++;
            size=spaces;
        }
        //build custom string
        tqbs=qbs_new(size,1);
        if (cr){
            tqbs->chr[0]=13; if (cr_size==2) tqbs->chr[1]=10;
            memset(&tqbs->chr[cr_size],32,spaces);
            }else{
            memset(tqbs->chr,32,spaces);
        }
        if (tab_LPRINT) sub__dest(tab_LPRINT_olddest);
        return tqbs;
    }
    
    qbs *func_spc(int32 spaces){
        if (new_error) return qbs_new(0,1);
        
        static qbs *tqbs;
        if ((spaces<-32768)||(spaces>32767)){tqbs=qbs_new(0,1); error(6); return tqbs;}//Overflow
        if (spaces<0) spaces=0;
        
        //for files, spc simply adds that many spaces
        if (tab_spc_cr_size==2){//files
            tqbs=qbs_new(spaces,1);
            memset(tqbs->chr,32,spaces);
            return tqbs;
        }
        
        static int32 tab_LPRINT_olddest;
        if (tab_LPRINT){
            if (!lprint_image) qbs_lprint(qbs_new(0,1),0);//send dummy data to init the LPRINT image
            tab_LPRINT_olddest=func__dest();
            sub__dest(lprint_image);
        }
        
        //for screens, spc adds that many spaces MOD screen_width_in_characters
        //if 2 rows are bridged, the top row's characters are not printed to, just the lower
        static int32 x,x2;
        //calc spaces remaining on current screen row & MOD
        static int32 spaces_left_on_line;
        static qbs *onespace=NULL; if (!onespace){onespace=qbs_new(1,0); onespace->chr[0]=32;}
        static int32 onespace_width;//for variable width fonts
        if (write_page->text){
            spaces_left_on_line=write_page->width-write_page->cursor_x+1;
            spaces%=write_page->width;//MOD
            }else{
            x=fontwidth[write_page->font]; 
            if (x){
                x2=write_page->width/x;//characters per row
                spaces_left_on_line=x2-write_page->cursor_x+1;
                spaces%=x2;//MOD
                }else{
                x2=write_page->width-write_page->cursor_x+1;//pixels left on row
                onespace_width=func__printwidth(onespace,NULL,0);
                spaces_left_on_line=x2/onespace_width;
                spaces%=(write_page->width/onespace_width);//MOD
            }
        }
        
        //build string
        if (spaces_left_on_line>=spaces){
            tqbs=qbs_new(spaces,1);
            memset(tqbs->chr,32,spaces);
            }else{
            spaces-=spaces_left_on_line;
            tqbs=qbs_new(1+spaces,1);
            tqbs->chr[0]=13;
            memset(tqbs->chr+1,32,spaces);
        }
        
        if (tab_LPRINT) sub__dest(tab_LPRINT_olddest);
        return tqbs;
    }
    
    float func_pmap(float val,int32 option){
        if (new_error) return 0;
        int32 e=0; float v=qb_gfx_pmap(write_page,val,option,&e);
        if (e){error(e); return 0;}
        return v;
    }
    
    
    
    uint32 func_screen(int32 y,int32 x,int32 returncol,int32 passed){
        
        static uint8 chr[65536];
        static int32 x2,y2,x3,y3,i,i2,i3;
        static uint32 col,firstcol;
        uint8 *cp;
        
        if (!passed) returncol=0;

        #ifdef QBNex_WINDOWS
            if (read_page->console){
                SECURITY_ATTRIBUTES SecAttribs = {sizeof(SECURITY_ATTRIBUTES), 0, 1};
                HANDLE cl_conout = CreateFileA("CONOUT$", GENERIC_READ | GENERIC_WRITE, FILE_SHARE_READ | FILE_SHARE_WRITE, & SecAttribs, OPEN_EXISTING, 0, 0);
                COORD cp1 = {x-1, y-1};
                DWORD t;
                uint16 a; 
                if (returncol){
                    int x1 = ReadConsoleOutputAttribute(cl_conout, &a, 1, cp1, &t) ;
                    return a;
                } else {
                    ReadConsoleOutputCharacterA(cl_conout, (char *) &a, 1, cp1, &t) ;
                    return a & 0xff;
                }
            }
        #endif
        
        if (read_page->text){
            //on screen?
            if ((y<1)||(y>read_page->height)){error(5); return 0;}
            if ((x<1)||(x>read_page->width)){error(5); return 0;}
            if (returncol){
                return read_page->offset[((y-1)*read_page->width+x-1)*2+1];
            }
            return read_page->offset[((y-1)*read_page->width+x-1)*2];
        }
        
        //only 8x8,8x14,8x16 supported
        if ((read_page->font!=8)&&(read_page->font!=14)&&(read_page->font!=16)){error(5); return 0;}
        
        //on screen?
        x2=read_page->width/fontwidth[read_page->font];
        y2=read_page->height/fontheight[read_page->font];
        if ((y<1)||(y>y2)){error(5); return 0;}
        if ((x<1)||(x>x2)){error(5); return 0;}
        
        //create "signature" of character on screen
        x--; y--;
        i=0;
        i3=1;
        y3=y*fontheight[read_page->font];
        for (y2=0;y2<fontheight[read_page->font];y2++){
            x3=x*fontwidth[read_page->font];
            for (x2=0;x2<fontwidth[read_page->font];x2++){
                col=point(x3,y3);
                if (col){
                    if (i3){i3=0;firstcol=col;}
                    col=255;
                }
                chr[i]=col;
                i++;
                x3++;
            }
            y3++;
        }
        
        if (i3){//assume SPACE, no non-zero pixels were found
            if (returncol) return 1;
            return 32;
        }
        
        i3=i;//number of bytes per character "signature"
        
        //compare signature with all ascii characters
        for (i=0;i<=255;i++){
            if (read_page->font==8) cp=&charset8x8[i][0][0];
            if (read_page->font==14) cp=&charset8x16[i][1][0];
            if (read_page->font==16) cp=&charset8x16[i][0][0];
            i2=memcmp(cp,chr,i3);
            if (!i2){//identical!
                if (returncol) return firstcol;
                return i;
            }
        }
        
        //no match found
        if (returncol) return 0;
        return 32;
    }
    
    void sub_bsave(qbs *filename,int32 offset,int32 size){
        if (new_error) return;
        static ofstream fh;
        
        static qbs *tqbs=NULL;
        if (!tqbs) tqbs=qbs_new(0,0);
        static qbs *nullt=NULL;
        if (!nullt) nullt=qbs_new(1,0);
        
        static int32 x;
        nullt->chr[0]=0;
        if ((offset<-65536)||(offset>65535)){error(6); return;}//Overflow
        offset&=0xFFFF;
        //note: QBNex allows a maximum of 65536 bytes, QB only allows 65535
        if ((size<-65536)||(size>65536)){error(6); return;}//Overflow
        if (size!=65536) size&=0xFFFF;
        qbs_set(tqbs,qbs_add(filename,nullt));//prepare null-terminated filename
        fh.open(fixdir(tqbs),ios::binary|ios::out);
        if (fh.is_open()==NULL){error(64); return;}//Bad file name
        x=253; fh.write((char*)&x,1);//signature byte
        x=(defseg-&cmem[0])/16; fh.write((char*)&x,2);//segment
        x=offset; fh.write((char*)&x,2);//offset
        x=size; if (x>65535) x=0;//if filesize>65542 then size=filesize-7
        fh.write((char*)&x,2);//size
        fh.write((char*)(defseg+offset),size);//data
        fh.close();
    }
    
    void sub_bload(qbs *filename,int32 offset,int32 passed){
        if (new_error) return;
        static uint8 header[7];
        static ifstream fh;
        static qbs *tqbs=NULL;
        if (!tqbs) tqbs=qbs_new(0,0);
        static qbs *nullt=NULL;
        if (!nullt) nullt=qbs_new(1,0);
        
        
        static int32 x,file_seg,file_off,file_size;
        nullt->chr[0]=0;
        if (passed){
            if ((offset<-65536)||(offset>65535)){error(6); return;}//Overflow
            offset&=0xFFFF;
        }
        qbs_set(tqbs,qbs_add(filename,nullt));//prepare null-terminated filename
        fh.open(fixdir(tqbs),ios::binary|ios::in);
        if (fh.is_open()==NULL){error(53); return;}//File not found
        fh.read((char*)header,7); if (fh.gcount()!=7) goto error;
        if (header[0]!=253) goto error;
        file_seg=header[1]+header[2]*256;
        file_off=header[3]+header[4]*256;
        file_size=header[5]+header[6]*256;
        if (file_size==0){
            fh.seekg(0,ios::end);
            file_size=fh.tellg();
            fh.seekg(7,ios::beg);
            file_size-=7;
            if (file_size<65536) file_size=0;
        }
        if (passed){
            fh.read((char*)(defseg+offset),file_size);
            }else{
            fh.read((char*)(&cmem[0]+file_seg*16+file_off),file_size);
        }
        if (fh.gcount()!=file_size) goto error;
        fh.close();
        return;
        error:
        fh.close();
        error(54);//Bad file mode
    }
    
    //LOF, EOF, SEEK, LOC, INPUT$(n,#f), LINE INPUT #: implemented in Rust
    //(crates/qb-runtime/src/fileio.rs); TCP/IP handles and the keyboard stay here.
    int64 func_lof(int32 i){
        if (i<0){//special handle?
            static special_handle_struct *sh; sh=(special_handle_struct*)list_get(special_handles,-(i+1)); if (!sh){error(52); return 0;}
            if (sh->type==1){//stream
                static stream_struct *st; st=(stream_struct*)sh->index;
                stream_update(st);
                return st->in_size;
            }//stream
            error(52); return 0;
        }//special handle
        int64 v; int32 e=qb_file_lof(i,&v); if (e) error(e);
        return v;
    }

    int32 func_eof(int32 i){
        if (i<0){//special handle?
            static special_handle_struct *sh; sh=(special_handle_struct*)list_get(special_handles,-(i+1)); if (!sh){error(52); return 0;}
            if (sh->type==1){//stream
                static stream_struct *st; st=(stream_struct*)sh->index;
                if (st->eof) return -1;
                return 0;
            }//stream
            error(52); return 0;
        }//special handle
        int32 v; int32 e=qb_file_eof(i,&v); if (e) error(e);
        return v;
    }

    void sub_seek(int32 i,int64 pos){
        if (new_error) return;
        int32 e=qb_file_seek_set(i,pos); if (e) error(e);
    }

    int64 func_seek(int32 i){
        int64 v; int32 e=qb_file_seek_get(i,&v); if (e) error(e);
        return v;
    }

    int64 func_loc(int32 i){
        int64 v; int32 e=qb_file_loc(i,&v); if (e) error(e);
        return v;
    }

    qbs *func_input(int32 n,int32 i,int32 passed){
        if (new_error) return qbs_new(0,1);
        static qbs *str,*str2;
        static int32 x;
        if (n<0) str=qbs_new(0,1); else str=qbs_new(n,1);
        if (passed){
            int32 len=str->len;
            int32 e=qb_file_input_n(n,i,str->chr,str->len,&len);
            str->len=len;
            if (e) error(e);
            return str;
            }else{
            //keyboard/piped
            //      For extended-two-byte character codes, only the first, CHR$(0), is returned and counts a 1 byte
            if (n<0){error(52); return str;}
            if (n==0) return str;
            x=0;
            waitforinput:
            str2=qbs_inkey();
            if (str2->len){
                str->chr[x]=str2->chr[0];
                x++;
            }
            qbs_free(str2);
            if (stop_program) return str;
            if (x<n){
                evnt(0);//check for new events
                Sleep(10);
                goto waitforinput;
            }
            return str;
        }
    }

    void sub_file_line_input_string(int32 fileno, qbs *deststr){
        if (new_error) return;
        const uint8 *out; int32 len=0,assign=0;
        int32 e=qb_file_line_input(fileno,&out,&len,&assign);
        if (assign) qbs_set(deststr,qbs_new_txt_len((const char*)out,len));
        if (e) error(e);
    }
    
    double func_sqr(double value){double r; if (int32 e=qb_sqr(value,&r)) error(e); return r;}
    
    
    
    
    
    #include "parts/audio/out/src.c"
    
    #ifdef DEPENDENCY_ZLIB
        #ifdef QBNex_BACKSLASH_FILESYSTEM
            #include "parts\\zlib\\src.c"
        #else
            #include "parts/zlib/src.c"
        #endif
    #endif
    
    
    
    
    
    
    
    qbs *func_command_str=NULL;
    char **func_command_array = NULL;
    int32 func_command_count = 0;

    //COMMAND$, SHELL family, KILL, NAME, CHDIR, MKDIR, RMDIR: implemented in Rust
    //(crates/qb-runtime/src/{env,shell,fsops}.rs). Full screen and console state stay here.
    qbs *func_command(int32 index, int32 passed){
        static qbs *tqbs;
        if (passed) { //Get specific parameter ("" when out of bounds)
            const uint8 *out; int32 len=0;
            qb_command_arg(func_command_array,func_command_count,index,&out,&len);
            tqbs=qbs_new(len,1);
            memcpy(tqbs->chr,out,len);
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

    static int32 shell_use_console(){
        return (console&&console_active)?1:0;
    }

    static int32 shell_leave_full_screen(){//exit full screen mode if necessary
        int32 mode=full_screen;
        if (mode){
            full_screen_set=0;
            do{ Sleep(0); }while(full_screen);
        }
        return mode;
    }

    static void shell_restore_full_screen(int32 mode){
        if (mode){
            full_screen_set=mode;
            do{ Sleep(0); }while(!full_screen);
        }
    }

    static int64 shell_run(qbs *str,int32 show,int32 wait,int32 use_console){
        int64 rc;
        if (wait) shell_call_in_progress=1;
        rc=qb_shell_run(str->chr,str->len,show,wait,use_console);
        if (wait) shell_call_in_progress=0;
        return rc;
    }

    static int64 shell_interactive(int32 use_console){
        int64 rc;
        shell_call_in_progress=1;
        rc=qb_shell_interactive(use_console);
        shell_call_in_progress=0;
        return rc;
    }

    int64 func_shell(qbs *str){
        if (new_error) return 1;
        int32 full_screen_mode=shell_leave_full_screen();
        int32 use_console=shell_use_console();
        int64 return_code=str->len?shell_run(str,1,1,use_console):shell_interactive(use_console);
        shell_restore_full_screen(full_screen_mode);
        return return_code;
    }

    int64 func__shellhide(qbs *str){ //func _SHELLHIDE(...
        if (new_error) return 1;
        if (!str->len){error(5); return 1;}//cannot launch a hidden console
        return shell_run(str,0,1,0);
    }

    void sub_shell(qbs *str,int32 passed){
        if (new_error) return;
        int32 full_screen_mode=shell_leave_full_screen();
        int32 use_console=shell_use_console();
        if (passed) if (str->len==0) passed=0;//"" means launch a CONSOLE
        if (passed) shell_run(str,1,1,use_console); else shell_interactive(use_console);
        shell_restore_full_screen(full_screen_mode);
    }

    void sub_shell2(qbs *str,int32 passed){ //HIDE
        if (new_error) return;
        if (passed&1){sub_shell4(str,passed&2); return;}
        if (!(passed&2)){error(5); return;}//should not hide a shell waiting for input
        if (!str->len){error(5); return;}//cannot launch a hidden console
        shell_run(str,0,1,0);
    }

    void sub_shell3(qbs *str,int32 passed){//_DONTWAIT
        //shell3 launches 'str' but does not wait for it to complete
        if (new_error) return;
        if (passed&1){sub_shell4(str,passed&2); return;}
        static qbs *cmd=NULL; if (!cmd) cmd=qbs_new_txt("cmd");
        if (!(passed&2)||!str->len) str=cmd;//no command: open a console window
        shell_run(str,1,0,0);
    }

    void sub_shell4(qbs *str,int32 passed){//_DONTWAIT & _HIDE
        //if passed&2 set a string was given
        if (!(passed&2)){error(5); return;}//should not hide a shell waiting for input
        if (!str->len){error(5); return;}//console unsupported
        shell_run(str,0,0,0);
    }

    void sub_kill(qbs *str){
        if (new_error) return;
        int32 e=qb_kill(str->chr,str->len); if (e) error(e);
    }

    void sub_name(qbs *oldname,qbs *newname){
        if (new_error) return;
        int32 e=qb_name(oldname->chr,oldname->len,newname->chr,newname->len); if (e) error(e);
    }

    void sub_chdir(qbs *str){
        if (new_error) return;
        int32 e=qb_chdir(str->chr,str->len); if (e) error(e);
    }

    void sub_mkdir(qbs *str){
        if (new_error) return;
        int32 e=qb_mkdir(str->chr,str->len); if (e) error(e);
    }

    void sub_rmdir(qbs *str){
        if (new_error) return;
        int32 e=qb_rmdir(str->chr,str->len); if (e) error(e);
    }
        
        long double pow2(long double x,long double y){
            if (x<0){
                if (y!=floor(y)){error(5); return 0;}
            }
            return pow(x,y);
        }
        
        int32 func_freefile(){
            return gfs_fileno_freefile();
        }
        
        void sub__mousehide(){
            #ifdef QBNex_GUI
                if (!screen_hide){
                    while (!window_exists){Sleep(100);}
                    qb_window_set_cursor(QB_CURSOR_HIDDEN);
                }
            #endif
        }

        int mouse_cursor_style=QB_CURSOR_DEFAULT;

        void sub__mouseshow(qbs *style, int32 passed){
            if (new_error) return;
            
            #ifdef QBNex_GUI
                
                static qbs *str=NULL; if (str==NULL) str=qbs_new(0,0);
                if (passed){
                    qbs_set(str,qbs_ucase(style));
                    if (qbs_equal(str,qbs_new_txt("DEFAULT"))) {mouse_cursor_style=QB_CURSOR_DEFAULT; goto cursor_valid;}
                    if (qbs_equal(str,qbs_new_txt("LINK"))) {mouse_cursor_style=QB_CURSOR_LINK; goto cursor_valid;}
                    if (qbs_equal(str,qbs_new_txt("TEXT"))) {mouse_cursor_style=QB_CURSOR_TEXT; goto cursor_valid;}
                    if (qbs_equal(str,qbs_new_txt("CROSSHAIR"))) {mouse_cursor_style=QB_CURSOR_CROSSHAIR; goto cursor_valid;}
                    if (qbs_equal(str,qbs_new_txt("VERTICAL"))) {mouse_cursor_style=QB_CURSOR_VERTICAL; goto cursor_valid;}
                    if (qbs_equal(str,qbs_new_txt("HORIZONTAL"))) {mouse_cursor_style=QB_CURSOR_HORIZONTAL; goto cursor_valid;}
                    if (qbs_equal(str,qbs_new_txt("TOPLEFT_BOTTOMRIGHT"))) {mouse_cursor_style=QB_CURSOR_TOPLEFT_BOTTOMRIGHT; goto cursor_valid;}
                    if (qbs_equal(str,qbs_new_txt ("TOPRIGHT_BOTTOMLEFT"))) {mouse_cursor_style=QB_CURSOR_TOPRIGHT_BOTTOMLEFT; goto cursor_valid;}
                    if (qbs_equal(str,qbs_new_txt ("WAIT"))) {mouse_cursor_style=QB_CURSOR_WAIT; goto cursor_valid;}
                    if (qbs_equal(str,qbs_new_txt ("HELP"))) {mouse_cursor_style=QB_CURSOR_HELP; goto cursor_valid;}
                    if(qbs_equal(str,qbs_new_txt("CYCLE"))||qbs_equal(str, qbs_new_txt("MOVE"))) {mouse_cursor_style=QB_CURSOR_MOVE; goto cursor_valid;}
                    error(5); return;
                }
                cursor_valid:
                
                if (!screen_hide){
                    while (!window_exists){Sleep(100);}
                    qb_window_set_cursor(mouse_cursor_style);
                }
                
            #endif
            
        }
        
        float func__mousemovementx(int32 context, int32 passed){
            int32 handle;
            handle=mouse_message_queue_default;
            if (passed) handle=context;
            mouse_message_queue_struct *queue=(mouse_message_queue_struct*)list_get(mouse_message_queue_handles,handle);
            if (queue==NULL){error(258); return 0;}
            return queue->queue[queue->current].movementx;
        }
        float func__mousemovementy(int32 context, int32 passed){
            int32 handle;
            handle=mouse_message_queue_default;
            if (passed) handle=context;    
            mouse_message_queue_struct *queue=(mouse_message_queue_struct*)list_get(mouse_message_queue_handles,handle);
            if (queue==NULL){error(258); return 0;}
            return queue->queue[queue->current].movementy;
        }
        
        void sub__mousemove(float x,float y){
            #ifdef QBNex_GUI
                int32 x2,y2,sx,sy;
                if (display_page->text){
                    sx=fontwidth[display_page->font]*display_page->width; sy=fontheight[display_page->font]*display_page->height;
                    if (x<0.5) goto error;
                    if (y<0.5) goto error;
                    if (x>((float)display_page->width)+0.5) goto error;
                    if (y>((float)display_page->height)+0.5) goto error;
                    x-=0.5; y-=0.5;
                    x=x*(float)fontwidth[display_page->font]; y=y*(float)fontheight[display_page->font];
                    x2=qbr_float_to_long(x); y2=qbr_float_to_long(y);
                    if (x2<0) x2=0;
                    if (y2<0) y2=0;
                    if (x2>sx-1) x2=sx-1;
                    if (y2>sy-1) y2=sy-1;
                    }else{
                    sx=display_page->width; sy=display_page->height;
                    x2=qbr_float_to_long(x); y2=qbr_float_to_long(y);
                    if (x2<0) goto error;
                    if (y2<0) goto error;
                    if (x2>sx-1) goto error;
                    if (y2>sy-1) goto error;
                }
                //x2,y2 are pixel co-ordinates
                //adjust for fullscreen position as necessary:
                x2*=x_scale; y2*=y_scale;
                x2+=x_offset; y2+=y_offset;
                while (!window_exists){Sleep(100);} 
                qb_window_warp_mouse(x2, y2);
                return;
                error:
                error(5);
            #endif
        }
        
        float func__mousex(int32 context, int32 passed){
            
            static int32 x,x2;
            static float f;
            
            int32 handle;

            #ifdef QBNex_WINDOWS
                if (read_page->console){return consolemousex;}
            #endif

            handle=mouse_message_queue_default;
            if (passed) handle=context;    
            mouse_message_queue_struct *queue=(mouse_message_queue_struct*)list_get(mouse_message_queue_handles,handle);
            if (queue==NULL){error(258); return 0;}
            x=queue->queue[queue->current].x;
            
            //calculate pixel offset of mouse within SCREEN using environment variables
            x-=environment_2d__screen_x1;
            x=qbr_float_to_long((((float)x+0.5f)/environment_2d__screen_x_scale)-0.5f);
            if (x<0) x=0;
            if (x>=environment_2d__screen_width) x=environment_2d__screen_width-1;
            
            //restrict range to the current display page's range to avoid causing errors
            x2=display_page->width; if (display_page->text) x2*=fontwidth[display_page->font];
            if (x>=x2) x=x2-1;
            
            if (display_page->text){
                f=x;
                x2=fontwidth[display_page->font];
                f=f/(float)x2+0.5f;
                x2=qbr_float_to_long(f);
                if (x2>x) f-=0.001f;
                if (x2<x) f+=0.001f;
                return floor(f + 0.5);
            }
            
            return x;
        }
        
        float func__mousey(int32 context, int32 passed){
            
            static int32 y,y2;
            static float f;
            
            int32 handle;

            #ifdef QBNex_WINDOWS
                if (read_page->console){return consolemousey;}
            #endif

            handle=mouse_message_queue_default;
            if (passed) handle=context;    
            mouse_message_queue_struct *queue=(mouse_message_queue_struct*)list_get(mouse_message_queue_handles,handle);
            if (queue==NULL){error(258); return 0;}
            y=queue->queue[queue->current].y;
            
            //calculate pixel offset of mouse within SCREEN using environment variables
            y-=environment_2d__screen_y1;
            y=qbr_float_to_long((((float)y+0.5f)/environment_2d__screen_y_scale)-0.5f);
            if (y<0) y=0;
            if (y>=environment_2d__screen_height) y=environment_2d__screen_height-1;
            
            //restrict range to the current display page's range to avoid causing errors
            y2=display_page->height; if (display_page->text) y2*=fontheight[display_page->font];
            if (y>=y2) y=y2-1;
            
            if (display_page->text){
                f=y;
                y2=fontheight[display_page->font];
                f=f/(float)y2+0.5f;
                y2=qbr_float_to_long(f);
                if (y2>y) f-=0.001f;
                if (y2<y) f+=0.001f;
                return floor(f + 0.5);
            }
            
            return y;
        }
        
        
        int32 func__mousepipeopen(){
            //creates a new mouse pipe, routing all mouse input into it before any preceeding pipes receive access to the data
            
            //create new queue
            int32 context=list_add(mouse_message_queue_handles);
            mouse_message_queue_struct *queue=(mouse_message_queue_struct*)list_get(mouse_message_queue_handles,context);
            initialize_mouse_message_queue(queue,65535);
            
            //link new queue to child queue
            int32 child_context=mouse_message_queue_first;
            mouse_message_queue_struct *child_queue=(mouse_message_queue_struct*)list_get(mouse_message_queue_handles,child_context);
            queue->child=child_context;
            child_queue->parent=context;
            
            //set new queue and primary queue
            mouse_message_queue_first=context;
            
            return context;
        }
        
        void sub__mouseinputpipe(int32 context){
            //pushes the current _MOUSEINPUT event to the lower pipe, effectively sharing the input with the lower pipe
            
            mouse_message_queue_struct *queue=(mouse_message_queue_struct*)list_get(mouse_message_queue_handles,context);
            if (queue==NULL){error(258); return;}
            
            if (context==mouse_message_queue_default){error(5); return;} //cannot pipe input from the default queue
            
            int32 child_context=queue->child;
            mouse_message_queue_struct *child_queue=(mouse_message_queue_struct*)list_get(mouse_message_queue_handles,child_context);
            
            //create new event in child queue
            int32 i=child_queue->last+1; if (i>child_queue->lastIndex) i=0;
            if (i==child_queue->current){
                int32 nextIndex=child_queue->last+1; if (nextIndex>child_queue->lastIndex) nextIndex=0;
                child_queue->current=nextIndex;
            }
            
            int32 i2=queue->current;
            
            //copy event to child queue
            child_queue->queue[i].x=queue->queue[i2].x;
            child_queue->queue[i].y=queue->queue[i2].y;
            child_queue->queue[i].movementx=queue->queue[i2].movementx;
            child_queue->queue[i].movementy=queue->queue[i2].movementy;
            child_queue->queue[i].buttons=queue->queue[i2].buttons;
            child_queue->last=i;
            
        }
        
        void sub__mousepipeclose(int32 context){
            //closes an existing pipe and reverts the new route the pipe created
            
            mouse_message_queue_struct *queue=(mouse_message_queue_struct*)list_get(mouse_message_queue_handles,context);
            if (queue==NULL){error(258); return;}
            if (context==mouse_message_queue_default){error(5); return;} //cannot delete default queue
            
            //todo!
            
        }
        
        
        int32 func__mouseinput(int32 context, int32 passed){
            int32 handle;
            handle=mouse_message_queue_default;
            if (passed) handle=context;    
            mouse_message_queue_struct *queue=(mouse_message_queue_struct*)list_get(mouse_message_queue_handles,handle);
            if (queue==NULL){error(258); return 0;}
            if (queue->current==queue->last) return 0;
            int32 newIndex=queue->current+1;
            if (newIndex>queue->lastIndex) newIndex=0;
            queue->current=newIndex;
            return -1;
        }
        
        int32 func__mousebutton(int32 i, int32 context, int32 passed){
            if (i<1){error(5); return 0;}
            #ifdef QBNex_WINDOWS
                if (read_page->console){ //console may support up to 5 mouse buttons according to the documentation.
                    if (i==1)return consolebutton&1;
                    if (i==2)return consolebutton&2;
                    if (i==3)return consolebutton&4;                                       
                    if (i==4)return consolebutton&8;
                    if (i==5)return consolebutton&16;
                    return 0;
                }
            #endif

            if (i>3) return 0;//current SDL only supports 3 mouse buttons!
            //swap indexes 2&3
            if (i==2){
                i=3;
                }else{
                if (i==3) i=2;
            }
            int32 handle;
            handle=mouse_message_queue_default;
            if (passed) handle=context;    
            mouse_message_queue_struct *queue=(mouse_message_queue_struct*)list_get(mouse_message_queue_handles,handle);
            if (queue==NULL){error(258); return 0;}
            if (queue->queue[queue->current].buttons&(1<<(i-1))) return -1;
            return 0;
        }
        
        int32 func__mousewheel(int32 context, int32 passed){
            static uint32 x;
            int32 handle;

            
            #ifdef QBNex_WINDOWS
                if (read_page->console){
                    if (consolebutton<-0x100)return -1;
                    if (consolebutton>0x100)return 1;
                    return 0;
                }
            #endif

            handle=mouse_message_queue_default;
            if (passed) handle=context;    
            mouse_message_queue_struct *queue=(mouse_message_queue_struct*)list_get(mouse_message_queue_handles,handle);
            if (queue==NULL){error(258); return 0;}
            x=queue->queue[queue->current].buttons;
            if ((x&(8+16))==(8+16)) return 0;//cancelled out change
            if (x&8) return -1;//up
            if (x&16) return 1;//down
            return 0;//no change
        }
        
        extern uint16 call_absolute_offsets[256];
        void call_absolute(int32 args,uint16 offset){
            memset(&cpu,0,sizeof(cpu_struct));//flush cpu
            cpu.cs=((defseg-cmem)>>4); cpu.ip=offset;
            cpu.ss=0xFFFF; cpu.sp=0;//sp "loops" to <65536 after first push
            cpu.ds=80;
            //push (near) arg offsets
            static int32 i;
            for (i=0;i<args;i++){
                cpu.sp-=2; *(uint16*)(cmem+cpu.ss*16+cpu.sp)=call_absolute_offsets[i];
            }
            //push ret segment, then push ret offset (both 0xFFFF to return control to QBNex)
            cpu.sp-=4; *(uint32*)(cmem+cpu.ss*16+cpu.sp)=0xFFFFFFFF;
            cpu_call();
        }
        
        void call_int(int32 i){
            
            if (i==0x33){
                
                if (cpu.ax==0){
                    cpu.ax=0xFFFF;//mouse installed
                    cpu.bx=2;
                    return;
                }
                
                if (cpu.ax==1){sub__mouseshow(NULL,0); return;}
                if (cpu.ax==2){sub__mousehide(); return;}
                if (cpu.ax==3){
                    //return the current mouse status
                    //buttons
                    
                    int32 handle;
                    handle=mouse_message_queue_default;    
                    mouse_message_queue_struct *queue=(mouse_message_queue_struct*)list_get(mouse_message_queue_handles,handle);
                    
                    //buttons
                    cpu.bx=queue->queue[queue->last].buttons&1;
                    if (queue->queue[queue->last].buttons&4) cpu.bx+=2;
                    
                    //x,y offsets    
                    static float mx,my;
                    
                    //temp override current message index to the most recent event
                    static int32 current_mouse_message_backup;
                    current_mouse_message_backup=queue->current;
                    queue->current=queue->last;
                    
                    mx=func__mousex(0,0); my=func__mousey(0,0);
                    
                    //restore "current" message index
                    queue->current=current_mouse_message_backup;
                    
                    cpu.cx=mx; cpu.dx=my;
                    //double x-axis value for modes 1,7,13
                    if ((display_page->compatible_mode==1)||(display_page->compatible_mode==7)||(display_page->compatible_mode==13)) cpu.cx*=2;
                    if (display_page->text){
                        //note: a range from 0 to columns*8-1 is returned regardless of the number of actual pixels
                        cpu.cx=(mx-0.5)*8.0;
                        if (cpu.cx>=(display_page->width*8)) cpu.cx=(display_page->width*8)-1;
                        //note: a range from 0 to rows*8-1 is returned regardless of the number of actual pixels
                        //obselete line of code: cpu.dx=(((float)cpu.dx)/((float)(display_page->height*fontheight[display_page->font])))*((float)(display_page->height*8));//(mouse_y/height_in_pixels)*(rows*8)
                        cpu.dx=(my-0.5)*8.0;
                        if (cpu.dx>=(display_page->height*8)) cpu.dx=(display_page->height*8)-1;
                    }
                    return;
                }
                
                if (cpu.ax==7){//horizontal min/max
                    return;
                }
                if (cpu.ax==8){//vertical min/max
                    return;
                }
                
                //MessageBox2(NULL,"Unknown MOUSE Sub-function","Call Interrupt Error",MB_OK|MB_SYSTEMMODAL);
                //exit(cpu.ax);
                
                return;
            }
            
        }
        
        
        
        
        
        //2D PROTOTYPE QBNex<->C CALLS
        
        //Creating/destroying an image surface:
        
        int32 func__newimage(int32 x,int32 y,int32 bpp,int32 passed){
            // #ifdef QBNex_WINDOWS
            // #if WINVER >= 0x0600 //this block is not compatible with XP
			// static bool j;
            // if(j != 1){
            //     FARPROC dpiaware;
            //     HMODULE user32 = LoadLibrary(TEXT("user32.dll"));
            //     if(user32 != NULL){
            //         dpiaware = GetProcAddress(user32, "SetProcessDPIAware");
            //         if(NULL != dpiaware){
            //             (dpiaware) ();
            //             j = 1;
			// 			FreeLibrary(user32);
            //         }
			// 		FreeLibrary(user32);
            //     }
            // }
			// #endif
            // #endif
            static int32 i;
            if (new_error) return 0;
            if (x<=0||y<=0){error(5); return 0;}
            if (!passed){
                bpp=write_page->compatible_mode;
                }else{
                i=0;
                if (bpp>=0&&bpp<=2) i=1;
                if (bpp>=7&&bpp<=13) i=1;
                if (bpp==256) i=1;
                if (bpp==32) i=1;
                if (!i){error(5); return 0;}
            }
            i=imgnew(x,y,bpp);
            if (!i) return -1;
            if (!passed){
                //adopt palette
                if (write_page->pal){
                    memcpy(img[i].pal,write_page->pal,1024);
                }
                //adopt font
                sub__font(write_page->font,-i,1);
                //adopt colors
                img[i].color=write_page->color;
                img[i].background_color=write_page->background_color;
                //adopt transparent color
                img[i].transparent_color=write_page->transparent_color;
                //adopt blend state
                img[i].alpha_disabled=write_page->alpha_disabled;
                //adopt print mode
                img[i].print_mode=write_page->print_mode;
            }
            return -i;
        }
        
        #ifdef QBNex_BACKSLASH_FILESYSTEM
            #include "parts\\video\\image\\src.c"
            #else
            #include "parts/video/image/src.c"
        #endif
        
        int32 func__copyimage(int32 i,int32 mode,int32 passed){
            static int32 i2,bytes,source_index;
            static img_struct *s,*d;
            if (new_error) return 0;
            s=resolve_page_or_image_ptr(i,source_index); if (!s) return 0;
            
            if (passed&1){
                if (mode!=s->compatible_mode){
                    if (mode!=33||s->compatible_mode!=32){error(5); return -1;}
                    //create new buffered hardware image
                    i2=new_hardware_img(s->width, s->height, (uint32*)s->offset32,
                    NEW_HARDWARE_IMG__BUFFER_CONTENT | NEW_HARDWARE_IMG__DUPLICATE_PROVIDED_BUFFER);
                    return i2+HARDWARE_IMG_HANDLE_OFFSET;
                }
            }
            
            //duplicate structure
            i2=newimg();
            s=&img[source_index];
            d=&img[i2];
            memcpy(d,s,sizeof(img_struct));
            //don't duplicate the memory lock (if any),
            //_MEMIMAGE needs to obtain a new lock for the copy
            img[i2].lock_id=NULL;
            img[i2].lock_offset=NULL;
            //duplicate pixel data
            bytes=d->width*d->height*d->bytes_per_pixel;
            qb_allocate_img_pixels(i2,bytes);
            if (!d->offset){freeimg(i2); return -1;}
            memcpy(d->offset,s->offset,bytes);
            d->flags|=IMG_FREEMEM;
            //duplicate palette
            if (d->pal){
                qb_allocate_img_palette(i2);
                if (!d->pal){qb_release_img_pixels(i2); freeimg(i2); return -1;}
                memcpy(d->pal,s->pal,1024);
                d->flags|=IMG_FREEPAL;
            }
            if (d->text && source_index < qb_text_utf8_cells.size() && i2 < qb_text_utf8_cells.size()){
                qb_text_utf8_cells[i2]=qb_text_utf8_cells[source_index];
                qb_text_utf8_cell_state[i2]=qb_text_utf8_cell_state[source_index];
            }
            //adjust flags
            if (d->flags&IMG_SCREEN)d->flags^=IMG_SCREEN;
            //return new handle
            return -i2;
        }
        
        void sub__freeimage(int32 i,int32 passed){
            if (new_error) return;
            if (passed){
                if (i>=0){//validate i
                    error(5); return;//The SCREEN's pages cannot be freed!
                    }else{
                    
                    static hardware_img_struct *himg;  
                    if (himg=get_hardware_img(i)){
                        flush_old_hardware_commands();
                        //add command to free image
                        //create new command handle & structure
                        int32 hgch=list_add(hardware_graphics_command_handles);
                        hardware_graphics_command_struct* hgc=(hardware_graphics_command_struct*)list_get(hardware_graphics_command_handles,hgch);
                        hgc->remove=0;
                        //set command values
                        hgc->command=HARDWARE_GRAPHICS_COMMAND__FREEIMAGE_REQUEST;
                        hgc->src_img=get_hardware_img_index(i);
                        himg->valid=0;
                        
                        //queue the command
                        hgc->next_command=0;
                        hgc->order=display_frame_order_next;
                        if (last_hardware_command_added){
                            hardware_graphics_command_struct* hgc2=(hardware_graphics_command_struct*)list_get(hardware_graphics_command_handles,last_hardware_command_added);
                            hgc2->next_command=hgch;
                        }
                        last_hardware_command_added=hgch;
                        if (first_hardware_command==0) first_hardware_command=hgch;
                        
                        return;
                    }
                    
                    if (!resolve_page_or_image_index(i,i)) return;
                }
                }else{
                i=write_page_index;
            }
            if (img[i].flags&IMG_SCREEN){error(5); return;}//The SCREEN's pages cannot be freed!
            if (write_page_index==i) sub__dest(-display_page_index);
            if (read_page_index==i) sub__source(-display_page_index);
            qb_release_img_owned_resources(i);
            freeimg(i);
        }
        
        void freeallimages(){
            static int32 i;
            //note: handles 0 & -1(1) are reserved
            for (i=2;i<nextimg;i++){
                if (img[i].valid && i != abs(console_image)){
                    if ((img[i].flags&IMG_SCREEN)==0){//The SCREEN's pages cannot be freed!
                        sub__freeimage(-i,1);
                    }
                }//valid
            }//i
        }
        
        //Selecting images:
        
        void sub__source(int32 i){ 
            if (new_error) return;
            if (!resolve_page_or_image_index(i,i)) return;
            select_read_page(i);
        }
        
        void sub__dest(int32 i){ 
            if (new_error) return;
            if (!resolve_page_or_image_index(i,i)) return;
            select_write_page(i);
        }
        
        int32 func__source(){
            return -read_page_index;
        }
        
        int32 func__dest(){
            return -write_page_index;
        }
        
        int32 func__display(){
            return -display_page_index;
        }
        
        //Changing the settings of an image surface:
        
        void sub__blend(int32 i,int32 passed){
            if (new_error) return;
            if (passed){
                if (i<0){
                    static hardware_img_struct *himg;  
                    if (himg=get_hardware_img(i)){
                        himg->alpha_disabled=0;
                        return;
                    }
                }
                if (!resolve_page_or_image_index(i,i)) return;
                }else{
                i=write_page_index;
            }
            if (img[i].bytes_per_pixel!=4){error(5); return;}
            img[i].alpha_disabled=0;
        }
        
        void sub__dontblend(int32 i,int32 passed){
            if (new_error) return;
            if (passed){
                if (i<0){
                    static hardware_img_struct *himg;  
                    if (himg=get_hardware_img(i)){
                        himg->alpha_disabled=1;
                        return;
                    }
                }
                if (!resolve_page_or_image_index(i,i)) return;
                }else{
                i=write_page_index;
            }
            if (img[i].bytes_per_pixel!=4) return;
            img[i].alpha_disabled=1;
        }
        
        
        void sub__clearcolor(uint32 c,int32 i,int32 passed){
            //--         _NONE->1       2       4
            if (new_error) return;
            if (passed&4){
                if (!resolve_page_or_image_index(i,i)) return;
                }else{
                i=write_page_index;
            }
            if (int32 e=qb_gfx_clearcolor(&img[i],c,passed)) error(e);
        }
        
        //Changing/Using an image surface:
        
        //_PUT "[(?,?)[-(?,?)]][,[?][,[?][,[(?,?)[-(?,?)]]]]]"
        //(defined elsewhere)
        
        //_IMGALPHA "?[,[?[{TO}?]][,?]]"
        void sub__setalpha(int32 a,uint32 c,uint32 c2,int32 i,int32 passed){
            //-->                             1        4        2
            if (new_error) return;
            if (passed&2){
                if (!resolve_page_or_image_index(i,i)) return;
                }else{
                i=write_page_index;
            }
            if (int32 e=qb_gfx_setalpha(&img[i],a,c,c2,passed)) error(e);
        }
        
        //Finding infomation about an image surface:
        
        int32 func__width(int32 i,int32 passed){
            if (new_error) return 0;

            #ifdef QBNex_WINDOWS
                if ((write_page->console && !passed)||i==console_image){
                    SECURITY_ATTRIBUTES SecAttribs = {sizeof(SECURITY_ATTRIBUTES), 0, 1};
                    HANDLE cl_conout = CreateFileA("CONOUT$", GENERIC_READ | GENERIC_WRITE, FILE_SHARE_READ | FILE_SHARE_WRITE, & SecAttribs, OPEN_EXISTING, 0, 0);
                    CONSOLE_SCREEN_BUFFER_INFO cl_bufinfo;
                    GetConsoleScreenBufferInfo(cl_conout, &cl_bufinfo);
                    return cl_bufinfo.srWindow.Right - cl_bufinfo.srWindow.Left + 1;
                }
            #endif
            #ifdef QBNex_UNIX
                if ((write_page->console && !passed)||i==console_image){
                    struct winsize w;
                    ioctl(STDOUT_FILENO, TIOCGWINSZ, &w);
                    return w.ws_col;
                }
            #endif

            if (passed){
                if (i<0){
                    static hardware_img_struct *himg;  
                    if (himg=get_hardware_img(i)){
                        return himg->w;
                    }
                }
                if (!resolve_page_or_image_index(i,i)) return 0;
                }else{
                i=write_page_index;
            }
            return img[i].width;
        }
        
        int32 func__height(int32 i,int32 passed){
            if (new_error) return 0;

            #ifdef QBNex_WINDOWS
                if ((write_page->console && !passed)||i==console_image){
                    SECURITY_ATTRIBUTES SecAttribs = {sizeof(SECURITY_ATTRIBUTES), 0, 1};
                    HANDLE cl_conout = CreateFileA("CONOUT$", GENERIC_READ | GENERIC_WRITE, FILE_SHARE_READ | FILE_SHARE_WRITE, & SecAttribs, OPEN_EXISTING, 0, 0);
                    CONSOLE_SCREEN_BUFFER_INFO cl_bufinfo;
                    GetConsoleScreenBufferInfo(cl_conout, &cl_bufinfo);
                    return cl_bufinfo.srWindow.Bottom - cl_bufinfo.srWindow.Top + 1;
                    return cl_bufinfo.dwMaximumWindowSize.Y;
                }
            #endif
            #ifdef QBNex_UNIX
                if ((write_page->console && !passed)||i==console_image){
                    struct winsize w;
                    ioctl(STDOUT_FILENO, TIOCGWINSZ, &w);
                    return w.ws_row;
                }
            #endif

            if (passed){
                if (i<0){
                    static hardware_img_struct *himg;  
                    if (himg=get_hardware_img(i)){
                        return himg->h;
                    }
                }
                if (!resolve_page_or_image_index(i,i)) return 0;
                }else{
                i=write_page_index;
            }
            return img[i].height;
        }
        
        int32 func__pixelsize(int32 i,int32 passed){
            if (new_error) return 0;
            if (passed){
                if (!resolve_page_or_image_index(i,i)) return 0;
                }else{
                i=write_page_index;
            }
            i=img[i].compatible_mode;
            if (i==32) return 4;
            if (!i) return 0;
            return 1;
        }
        
        int32 func__clearcolor(int32 i,int32 passed){
            if (new_error) return 0;
            if (passed){
                if (!resolve_page_or_image_index(i,i)) return 0;
                }else{
                i=write_page_index;
            }
            if (img[i].text) return -1;
            if (img[i].compatible_mode==32) return 0;
            return img[i].transparent_color;
        }
        
        int32 func__blend(int32 i,int32 passed){
            if (new_error) return 0;
            if (passed){
                if (!resolve_page_or_image_index(i,i)) return 0;
                }else{
                i=write_page_index;
            }
            if (img[i].compatible_mode==32){
                if (!img[i].alpha_disabled) return -1;
            }
            return 0;
        }
        
        uint32 func__defaultcolor(int32 i,int32 passed){
            if (new_error) return 0;
            if (passed){
                if (!resolve_page_or_image_index(i,i)) return 0;
                }else{
                i=write_page_index;
            }
            return img[i].color;
        }
        
        uint32 func__backgroundcolor(int32 i,int32 passed){
            if (new_error) return 0;
            if (passed){
                if (!resolve_page_or_image_index(i,i)) return 0;
                }else{
                i=write_page_index;
            }
            return img[i].background_color;
        }
        
        //Working with 256 color palettes:
        
        uint32 func__palettecolor(int32 n,int32 i,int32 passed){
            if (new_error) return 0;
            if (passed){
                if (!resolve_page_or_image_index(i,i)) return 0;
                }else{
                i=write_page_index;
            }
            int32 e=0; uint32 v=qb_gfx_palettecolor_get(&img[i],n,&e);
            if (e){error(e); return 0;}
            return v;
        }
        
        void sub__palettecolor(int32 n,uint32 c,int32 i,int32 passed){
            if (new_error) return;
            if (passed){
                if (!resolve_page_or_image_index(i,i)) return;
                }else{
                i=write_page_index;
            }
            if (int32 e=qb_gfx_palettecolor_set(&img[i],n,c)) error(e);
        }
        
        void sub__copypalette(int32 i,int32 i2,int32 passed){
            if (new_error) return;
            if (passed&1){
                if (!resolve_page_or_image_index(i,i)) return;
                }else{
                i=read_page_index;
            }
            if (!img[i].pal){error(5); return;}
            swap(i,i2);
            if (passed&2){
                if (!resolve_page_or_image_index(i,i)) return;
                }else{
                i=write_page_index;
            }
            if (!img[i].pal){error(5); return;}
            swap(i,i2);
            memcpy(img[i2].pal,img[i].pal,1024);
        }
        
        
        
        void sub__printstring(float x,float y,qbs* text,int32 i,int32 passed){
            if (new_error) return;

            int32 old_dest = func__dest();

            if (passed&2){
                sub__dest(i);
                if (!resolve_page_or_image_index(i,i)) goto printstring_exit;
                }else{
                i=write_page_index;
            }
            static img_struct *im;
            im=&img[i];
            if (!text->len) goto printstring_exit;
            if (im->text){
                int oldx = func_pos(0), oldy = func_csrlin();
                qbg_sub_locate(y, x, 0, 0, 0, 3);
                qbs_print(text, 0);
                qbg_sub_locate(oldy, oldx, 0, 0, 0, 3);
                goto printstring_exit;
            }
            //graphics modes only
            if (!text->len) goto printstring_exit;
            //Step?
            if (passed&1){im->x+=x; im->y+=y;}else{im->x=x; im->y=y;}
            //Adjust co-ordinates for viewport?
            static int32 x2,y2;
            if (im->clipping_or_scaling){
                if (im->clipping_or_scaling==2){
                    x2=qbr_float_to_long(im->x*im->scaling_x+im->scaling_offset_x)+im->view_offset_x;
                    y2=qbr_float_to_long(im->y*im->scaling_y+im->scaling_offset_y)+im->view_offset_y;
                    }else{
                    x2=qbr_float_to_long(im->x)+im->view_offset_x; y2=qbr_float_to_long(im->y)+im->view_offset_y;
                }
                }else{
                x2=qbr_float_to_long(im->x); y2=qbr_float_to_long(im->y); 
            }
            
            if (!text->len) goto printstring_exit;
            
            
            static uint32 w,h,z,z2,z3,a,a2,a3,color,background_color,f;
            static uint32 *lp;
            static uint8 *cp;
            
            color=im->color;
            background_color=im->background_color;
            
            f=im->font;
            h=fontheight[f];
            
            if (f>=32){//custom font
                
                //8-bit / alpha-disabled 32-bit / dont-blend(alpha may still be applied)
                if ((im->bytes_per_pixel==1)||((im->bytes_per_pixel==4)&&(im->alpha_disabled))||(fontflags[f]&8)){
                    
                    //render character
                    static int32 ok;
                    static uint8 *rt_data;
                    static int32 rt_w,rt_h,rt_pre_x,rt_post_x;
                    //int32 FontRenderTextASCII(int32 i,uint8*codepoint,int32 codepoints,int32 options,
                    //                          uint8**out_data,int32*out_x,int32 *out_y,int32*out_x_pre_increment,int32*out_x_post_increment){
                    ok=FontRenderTextASCII(font[f],(uint8*)text->chr,text->len,1,
                    &rt_data,&rt_w,&rt_h,&rt_pre_x,&rt_post_x);
                    if (!ok) goto printstring_exit;
                    
                    w=rt_w;
                    
                    switch(im->print_mode){
                        case 3:
                        for (y2=0;y2<h;y2++){
                            cp=rt_data+y2*w;
                            for (x2=0;x2<w;x2++){
                                if (*cp++) pset_and_clip(x+x2,y+y2,color); else pset_and_clip(x+x2,y+y2,background_color);
                            }}
                            break;
                            case 1:
                            for (y2=0;y2<h;y2++){
                                cp=rt_data+y2*w;
                                for (x2=0;x2<w;x2++){
                                    if (*cp++) pset_and_clip(x+x2,y+y2,color);
                                }}
                                break;
                                case 2:
                                for (y2=0;y2<h;y2++){
                                    cp=rt_data+y2*w;
                                    for (x2=0;x2<w;x2++){
                                        if (!(*cp++)) pset_and_clip(x+x2,y+y2,background_color);
                                    }}
                                    break;
                                    default:
                                    break;
                    }
                    
                    free(rt_data);
                    goto printstring_exit;
                }//1-8 bit
                //assume 32-bit blended
                
                a=(color>>24)+1; a2=(background_color>>24)+1;
                z=color&0xFFFFFF; z2=background_color&0xFFFFFF;
                
                //render character
                static int32 ok;
                static uint8 *rt_data;
                static int32 rt_w,rt_h,rt_pre_x,rt_post_x;
                //int32 FontRenderTextASCII(int32 i,uint8*codepoint,int32 codepoints,int32 options,
                //                          uint8**out_data,int32*out_x,int32 *out_y,int32*out_x_pre_increment,int32*out_x_post_increment){
                ok=FontRenderTextASCII(font[f],(uint8*)text->chr,text->len,0,
                &rt_data,&rt_w,&rt_h,&rt_pre_x,&rt_post_x);
                
                if (!ok) goto printstring_exit;
                
                w=rt_w;
                
                switch(im->print_mode){
                    case 3:
                    
                    static float r1,g1,b1,alpha1,r2,g2,b2,alpha2;
                    alpha1=(color>>24)&255; r1=(color>>16)&255; g1=(color>>8)&255; b1=color&255;
                    alpha2=(background_color>>24)&255; r2=(background_color>>16)&255; g2=(background_color>>8)&255; b2=background_color&255;
                    static float dr,dg,db,da;
                    
                    dr=r2-r1;
                    dg=g2-g1;
                    db=b2-b1;
                    da=alpha2-alpha1;
                    static float cw;//color weight multiplier, avoids seeing black when transitioning from RGBA(?,?,?,255) to RGBA(0,0,0,0)
                    if (alpha1) cw=alpha2/alpha1; else cw=100000;
                    static float d;
                    
                    for (y2=0;y2<h;y2++){
                        cp=rt_data+y2*w;
                        for (x2=0;x2<w;x2++){
                            
                            d=*cp++;
                            d=255-d;
                            d/=255.0;
                            static float r3,g3,b3,alpha3;
                            alpha3=alpha1+da*d;
                            d*=cw; if (d>1.0) d=1.0;
                            r3=r1+dr*d;
                            g3=g1+dg*d;
                            b3=b1+db*d;
                            static int32 r4,g4,b4,alpha4;
                            r4=qbr_float_to_long(r3);
                            g4=qbr_float_to_long(g3);
                            b4=qbr_float_to_long(b3);
                            alpha4=qbr_float_to_long(alpha3);
                            pset_and_clip(x+x2,y+y2,b4+(g4<<8)+(r4<<16)+(alpha4<<24));
                            
                        }}
                        break;
                        case 1:
                        for (y2=0;y2<h;y2++){
                            cp=rt_data+y2*w;
                            for (x2=0;x2<w;x2++){
                                z3=*cp++;
                                if (z3) pset_and_clip(x+x2,y+y2,((z3*a)>>8<<24)+z);
                            }}
                            break;
                            case 2:
                            for (y2=0;y2<h;y2++){
                                cp=rt_data+y2*w;
                                for (x2=0;x2<w;x2++){
                                    z3=*cp++;
                                    if (z3!=255) pset_and_clip(x+x2,y+y2,(((255-z3)*a2)>>8<<24)+z2);
                                }}
                                break;
                                default:
                                break;
                }
                free(rt_data);
                goto printstring_exit;
            }//custom font
            
            //default fonts
            static int32 character,character_c;
            for (character_c=0;character_c<text->len;character_c++){
                character=text->chr[character_c];
                if (im->font==8) cp=&charset8x8[character][0][0];
                if (im->font==14) cp=&charset8x16[character][1][0];
                if (im->font==16) cp=&charset8x16[character][0][0];
                switch(im->print_mode){
                    case 3:
                    for (y2=0;y2<h;y2++){ for (x2=0;x2<8;x2++){
                        if (*cp++) pset_and_clip(x+x2,y+y2,color); else pset_and_clip(x+x2,y+y2,background_color);
                    }}
                    break;
                    case 1:
                    for (y2=0;y2<h;y2++){ for (x2=0;x2<8;x2++){
                        if (*cp++) pset_and_clip(x+x2,y+y2,color);
                    }}
                    break;
                    case 2:
                    for (y2=0;y2<h;y2++){ for (x2=0;x2<8;x2++){
                        if (!(*cp++)) pset_and_clip(x+x2,y+y2,background_color);
                    }}
                    break;
                    default:
                    break;
                }
                x+=8;
            }//z

            printstring_exit:
            if (passed&2) sub__dest(old_dest);
            return;
        }
        
        int32 func__fontwidth(int32 f,int32 passed);
        int32 func__fontheight(int32 f,int32 passed);
        int32 func__font(int32 f,int32 passed);
        
        int32 func__printwidth(qbs* text, int32 screenhandle, int32 passed){
            /* Luke Ceddia <flukiluke@gmail.com>
                * This routine should be rewritten properly by calling True Type.
                * This is a temporary implementation
            */
            //Validate screenhandle (taken from func__font)
            if (passed) {
                if (screenhandle >= 0) {
                    validatepage(screenhandle);
                    screenhandle = page[screenhandle];
                }
                else {
                    screenhandle = -screenhandle; 
                    if (screenhandle >= nextimg) {
                        error(258); 
                        return 0;
                    } 
                    if (!img[screenhandle].valid) {
                        error(258); 
                        return 0;
                    }
                }
            }
            else {
                screenhandle = write_page_index;
            }
            if (img[screenhandle].text) { //Return LEN(text) in non-graphic modes
                //error(5);
                return text->len;
            }
            if (text->len == 0) return 0; //No point calculating an empty string
            int32 fonthandle = img[screenhandle].font; //Get the font used in screenhandle
            int32 fwidth = func__fontwidth(fonthandle, 1); //Try and get the font width
            if (fwidth != 0) return fwidth*(text->len); //if it's not a variable width font, return the width * the letter count
            int32 fheight = func__fontheight(fonthandle, 1); //Height of the font used in screenhandle
            int32 tempscreen = func__newimage(65535, fheight, 32, 1); //Just like calling _NEWIMAGE
            int32 oldwritepage = func__dest();
            sub__dest(tempscreen);
            int32 oldreadpage = func__source();
            sub__source(tempscreen);
            sub__font(fonthandle, NULL, 0); //Set the font on our new screen
            qbg_sub_color(0xffffffff, 0xffffffff, 0, NULL);
            qbs_print(text, 0);
            int xpos = 0;
            for (int i = 65534; i >= 0; i--) if (point(i, 0) != 0) {xpos = i; break;}
            sub__freeimage(tempscreen, 1);
            sub__dest(oldwritepage);
            sub__source(oldreadpage);
            if (xpos == 0) return 0;
            return xpos + 1;
        }
        
        /*int32 func__printwidth(qbs* text,int32 i,int32 passed){
            if (new_error) return 0;
            
            static int32 f;
            static img_struct *im;
            
            if (passed){
            if (i>=0){//validate i
            validatepage(i); i=page[i];
            }else{
            i=-i; if (i>=nextimg){error(258); return 0;} if (!img[i].valid){error(258); return 0;}
            }
            }else{
            i=write_page_index;
            }
            im=&img[i];
            if (im->text){error(5); return 0;}//graphics modes only
            if (!text->len) return 0;
            
            if (f>=32){//custom font
            //8-bit / alpha-disabled 32-bit / dont-blend(alpha may still be applied)
            if ((im->bytes_per_pixel==1)||((im->bytes_per_pixel==4)&&(im->alpha_disabled))||(fontflags[f]&8)){
            //render
            static int32 ok;
            static uint8 *rt_data;
            static int32 rt_w,rt_h,rt_pre_x,rt_post_x;
            //int32 FontRenderTextASCII(int32 i,uint8*codepoint,int32 codepoints,int32 options,
            //                          uint8**out_data,int32*out_x,int32 *out_y,int32*out_x_pre_increment,int32*out_x_post_increment){
            ok=FontRenderTextASCII(font[f],(uint8*)text->chr,text->len,1,
            &rt_data,&rt_w,&rt_h,&rt_pre_x,&rt_post_x);
            if (!ok) return 0;
            free(rt_data);
            return rt_w;
            }//1-8 bit
            //assume 32-bit blended
            //render
            static int32 ok;
            static uint8 *rt_data;
            static int32 rt_w,rt_h,rt_pre_x,rt_post_x;
            //int32 FontRenderTextASCII(int32 i,uint8*codepoint,int32 codepoints,int32 options,
            //                          uint8**out_data,int32*out_x,int32 *out_y,int32*out_x_pre_increment,int32*out_x_post_increment){
            ok=FontRenderTextASCII(font[f],(uint8*)text->chr,text->len,0,
            &rt_data,&rt_w,&rt_h,&rt_pre_x,&rt_post_x);
            if (!ok) return 0;
            free(rt_data);
            return rt_w;
            }//custom font
            
            //default fonts
            return text->len*8;
            
        }//printwidth*/
        
        
        int32 func__loadfont(qbs *f,int32 size,qbs *requirements,int32 passed){
            //f=_LOADFONT(ttf_filename$,height[,"bold,italic,underline,monospace,dontblend,unicode"])
            
            if (new_error) return NULL;
            
            qbs *s1=NULL; s1=qbs_new(0,0);
            qbs *req=NULL; req=qbs_new(0,0);
            qbs *s3=NULL; s3=qbs_new(0,0);
            uint8 r[32];
            int32 i,i2,i3;
            static int32 recall;
            
            //validate size
            if (size<1){error(5); return NULL;}
            if (size>2048) return -1;


            //load the file
            if (!f->len) return -1;//return invalid handle if null length string
            int32 fh,result;
            int64 bytes;
            fh=gfs_open(f,1,0,0);
            
            #ifdef QBNex_WINDOWS //rather than just immediately tossing an error, let's try looking in the default OS folder for the font first in case the user left off the filepath.
                if (fh<0&&recall==0) {
                    recall=-1; //to set a flag so we don't get trapped endlessly recalling the routine when the font actually doesn't exist
                    i=func__loadfont(qbs_add(qbs_new_txt("C:/Windows/Fonts/"),f), size, requirements,passed); //Look in the default windows font location
                    return i;
                }
            #endif
            recall=0;
            if (fh<0) return -1; //If we still can't load the font, then we just can't load the font...  Send an error code back.

            //check requirements
            memset(r,0,32);
            if (passed){
                if (requirements->len){
                    i=1;
                    qbs_set(req,qbs_ucase(requirements));//convert tmp str to perm str
                    nextrequirement:
                    i2=func_instr(i,req,qbs_new_txt(","),1);
                    if (i2){
                        qbs_set(s1,func_mid(req,i,i2-i,1));
                        }else{
                        qbs_set(s1,func_mid(req,i,req->len-i+1,1));
                    }
                    qbs_set(s1,qbs_rtrim(qbs_ltrim(s1)));
                    if (qbs_equal(s1,qbs_new_txt("BOLD"))){r[0]++; goto valid;}
                    if (qbs_equal(s1,qbs_new_txt("ITALIC"))){r[1]++; goto valid;}
                    if (qbs_equal(s1,qbs_new_txt("UNDERLINE"))){r[2]++; goto valid;}
                    if (qbs_equal(s1,qbs_new_txt("DONTBLEND"))){r[3]++; goto valid;}
                    if (qbs_equal(s1,qbs_new_txt("MONOSPACE"))){r[4]++; goto valid;}
                    if (qbs_equal(s1,qbs_new_txt("UNICODE"))){r[5]++; goto valid;}
                    error(5); return NULL;//invalid requirements
                    valid:
                    if (i2){i=i2+1; goto nextrequirement;}
                    for (i=0;i<32;i++) if (r[i]>1){error(5); return NULL;}//cannot define requirements twice
                }//->len
            }//passed
            int32 options;
            options=r[0]+(r[1]<<1)+(r[2]<<2)+(r[3]<<3)+(r[4]<<4)+(r[5]<<5);
            //1 bold TTF_STYLE_BOLD
            //2 italic TTF_STYLE_ITALIC
            //4 underline TTF_STYLE_UNDERLINE
            //8 dontblend (blending is the default in 32-bit alpha-enabled modes)
            //16 monospace
            //32 unicode

            bytes=gfs_lof(fh);
            static std::vector<uint8> content_storage;
            content_storage.resize(static_cast<size_t>(bytes));
            uint8 *content=content_storage.empty()?NULL:content_storage.data();
            if (!content){gfs_close(fh); return -1;}
            result=gfs_read(fh,-1,content,bytes);
            gfs_close(fh);
            if (result<0) return -1;
            
            //open the font
            //get free font handle
            for (i=32;i<=lastfont;i++){
                if (!font[i]) goto got_font_index;
            }
            //increase handle range
            lastfont++;
            font.resize(lastfont+1,0); font[lastfont]=NULL;
            fontheight.resize(lastfont+1,0);
            fontwidth.resize(lastfont+1,0);
            fontflags.resize(lastfont+1,0);
            i=lastfont;
            got_font_index:
            static int32 h;
            h=FontLoad(content,bytes,size,-1,options);
            if (!h) return -1;
            
            font[i]=h;
            fontheight[i]=size;
            fontwidth[i]=FontWidth(h);
            fontflags[i]=options;
            return i;
        }
        
        int32 fontopen(qbs *f,int32 size,int32 options){//Note: Just a stripped down verions of FUNC__LOADFONT
            
            static int32 i;
            
            //load the file
            if (!f->len) return -1;//return invalid handle if null length string
            static int32 fh,result;
            static int64 bytes;
            fh=gfs_open(f,1,0,0);
            if (fh<0) return -1;
            bytes=gfs_lof(fh);
            static std::vector<uint8> content_storage;
            content_storage.resize(static_cast<size_t>(bytes));
            uint8 *content=content_storage.empty()?NULL:content_storage.data();
            if (!content){gfs_close(fh); return -1;}
            result=gfs_read(fh,-1,content,bytes);
            gfs_close(fh);
            if (result<0) return -1;
            
            //open the font
            //get free font handle
            for (i=32;i<=lastfont;i++){
                if (!font[i]) goto got_font_index;
            }
            //increase handle range
            lastfont++;
            font.resize(lastfont+1,0); font[lastfont]=NULL;
            fontheight.resize(lastfont+1,0);
            fontwidth.resize(lastfont+1,0);
            fontflags.resize(lastfont+1,0);
            i=lastfont;
            got_font_index:
            static int32 h;
            h=FontLoad(content,bytes,size,-1,options);
            if (!h) return -1;
            
            font[i]=h;
            fontheight[i]=size;
            fontwidth[i]=FontWidth(h);
            fontflags[i]=options;
            return i;
        }
        
        
        
        
        void sub__font(int32 f,int32 i,int32 passed){
            //_FONT "?[,?]"
            static int32 i2;
            static img_struct *im;
            if (new_error) return;
            if (passed&1){
                if (!resolve_page_or_image_index(i,i)) return;
                }else{
                i=write_page_index;
            }
            im=&img[i];
            //validate f
            i2=0;
            if (f==8) i2=1;
            if (f==9) i2=1;
            if (f==14) i2=1;
            if (f==15) i2=1;
            if (f==16) i2=1;
            if (f==17) i2=1;
            if (f>=32&&f<=lastfont){
                if (font[f]) i2=1;
            }
            if (!i2){error(258); return;}
            
            if (im->text&&((fontflags[f]&16)==0)){error(5); return;}//only monospace fonts can be used on text surfaces
            //note: font changes to text screen mode images requires:
            //      i) font change across all screen pages
            //      ii) locking of the display
            //      iii) update of the data being displayed
            if (im->text){
                if (im->flags&IMG_SCREEN){
                    //lock display
                    if (autodisplay){
                        if (lock_display==0) lock_display=1;//request lock
                        while (lock_display!=2) Sleep(0);
                    }
                    //force update of data
                    screen_last_valid=0;//ignore cache used to update the screen on next update
                    //apply change across all video pages
                    for(i=0;i<pages;i++){
                        if(page[i]){
                            im=&img[page[i]];
                            im->font=f;
                            //note: moving the cursor is unnecessary
                        }
                    }
                    //unlock
                    if (autodisplay){
                        if (lock_display_required) lock_display=0;//release lock
                    }
                    return;
                }
            }//text
            
            im->font=f;
            im->cursor_x=1; im->cursor_y=1;
            im->top_row=1;
            if (im->compatible_mode) im->bottom_row=im->height/fontheight[f]; else im->bottom_row=im->height;
            im->bottom_row--; if (im->bottom_row<=0) im->bottom_row=1;
            return;
        }
        
        int32 func__fontwidth(int32 f,int32 passed){
            static int32 i2;
            if (new_error) return 0;
            if (passed){
                //validate f
                i2=0;
                if (f==8) i2=1;
                if (f==14) i2=1;
                if (f==16) i2=1;
                if (f>=32&&f<=lastfont){
                    if (font[f]) i2=1;
                }
                if (!i2){error(258); return 0;}
                }else{
                f=write_page->font;
            }
            return fontwidth[f];
        }
        
        int32 func__fontheight(int32 f,int32 passed){
            static int32 i2;
            if (new_error) return 0;
            if (passed){
                //validate f
                i2=0;
                if (f==8) i2=1;
                if (f==14) i2=1;
                if (f==16) i2=1;
                if (f>=32&&f<=lastfont){
                    if (font[f]) i2=1;
                }
                if (!i2){error(258); return 0;}
                }else{
                f=write_page->font;
            }
            return fontheight[f];
        }
        
        int32 func__font(int32 i,int32 passed){
            if (new_error) return 0;
            if (passed){
                if (!resolve_page_or_image_index(i,i)) return 0;
                }else{
                i=write_page_index;
            }
            return img[i].font;
        }
        
        void sub__freefont(int32 f){
            if (new_error) return;
            static int32 i,i2;
            //validate f (cannot remove QBASIC built in fonts!)
            i2=0;
            if (f>=32&&f<=lastfont){
                if (font[f]) i2=1;
            }
            if (!i2){error(258); return;}
            //check all surfaces, no surface can be using the font
            for (i=1;i<nextimg;i++){
                if (img[i].valid){
                    if (img[i].font==f){error(5); return;}
                }
            }
            //remove font
            FontFree(font[f]);
            font[f]=NULL;
        }
        
        void sub__printmode(int32 mode,int32 i,int32 passed){
            if (new_error) return;
            if (passed){
                if (!resolve_page_or_image_index(i,i)) return;
                }else{
                i=write_page_index;
            }
            if (img[i].text){
                if (mode!=1){error(5); return;}
            }
            if (mode==1) img[i].print_mode=3;//fill
            if (mode==2) img[i].print_mode=1;//keep
            if (mode==3) img[i].print_mode=2;//only
        }
        
        int32 func__printmode(int32 i,int32 passed){
            if (new_error) return 0;
            if (passed){
                if (!resolve_page_or_image_index(i,i)) return 0;
                }else{
                i=write_page_index;
            }
            return img[i].print_mode;
        }
        
        
        uint32 matchcol(int32 r,int32 g,int32 b){return qb_gfx_matchcol(write_page,r,g,b);}
        
        uint32 matchcol(int32 r,int32 g,int32 b,int32 i){return qb_gfx_matchcol(&img[i],r,g,b);}
        
        uint32 func__rgb(int32 r,int32 g,int32 b,int32 i,int32 passed){
            if (new_error) return 0;
            img_struct *im=write_page;
            if (passed){int32 k; if (!resolve_page_or_image_index(i,k)) return 0; im=&img[k];}
            return qb_gfx_rgb(im,r,g,b);
        }//rgb
        
        uint32 func__rgba(int32 r,int32 g,int32 b,int32 a,int32 i,int32 passed){
            if (new_error) return 0;
            img_struct *im=write_page;
            if (passed){int32 k; if (!resolve_page_or_image_index(i,k)) return 0; im=&img[k];}
            return qb_gfx_rgba(im,r,g,b,a);
        }//rgba
        
        int32 func__alpha(uint32 col,int32 i,int32 passed){
            if (new_error) return 0;
            img_struct *im=write_page;
            if (passed){int32 k; if (!resolve_page_or_image_index(i,k)) return 0; im=&img[k];}
            int32 e=0; int32 v=qb_gfx_component(im,col,3,&e);
            if (e){error(e); return 0;}
            return v;
        }
        
        int32 func__red(uint32 col,int32 i,int32 passed){
            if (new_error) return 0;
            img_struct *im=write_page;
            if (passed){int32 k; if (!resolve_page_or_image_index(i,k)) return 0; im=&img[k];}
            int32 e=0; int32 v=qb_gfx_component(im,col,0,&e);
            if (e){error(e); return 0;}
            return v;
        }
        
        int32 func__green(uint32 col,int32 i,int32 passed){
            if (new_error) return 0;
            img_struct *im=write_page;
            if (passed){int32 k; if (!resolve_page_or_image_index(i,k)) return 0; im=&img[k];}
            int32 e=0; int32 v=qb_gfx_component(im,col,1,&e);
            if (e){error(e); return 0;}
            return v;
        }
        
        int32 func__blue(uint32 col,int32 i,int32 passed){
            if (new_error) return 0;
            img_struct *im=write_page;
            if (passed){int32 k; if (!resolve_page_or_image_index(i,k)) return 0; im=&img[k];}
            int32 e=0; int32 v=qb_gfx_component(im,col,2,&e);
            if (e){error(e); return 0;}
            return v;
        }
        
        void sub_end(){
            
            if (sub_gl_called) error(271);
            
            dont_call_sub_gl=1;
            
            sub_close(NULL,0);
            exit_blocked=0;//allow exit via X-box or CTRL+BREAK

            
            #ifdef DEPENDENCY_CONSOLE_ONLY
                screen_hide=1;
            #endif
            
            if (!screen_hide){
                //1. set the display page as the destination page
                sub__dest(func__display());
                //2. VIEW PRINT bottomline,bottomline
                static int32 y;
                if (write_page->text){
                    y=write_page->height;
                    }else{
                    y=write_page->height/fontheight[write_page->font];
                }
                qbg_sub_view_print(y,y,1|2);
                //3. PRINT 'clears the line without having to worry about its contents/size
                qbs_print(nothingstring,1);
                //4. PRINT "Press any key to continue"
                qbs_print(qbs_new_txt("Press any key to continue"),0);
                //5. Clear any buffered keypresses
                static uint32 qbs_tmp_base;
                qbs_tmp_base=qbs_tmp_list_nexti;
                while(qbs_cleanup(qbs_tmp_base,qbs_notequal(qbs_inkey(),qbs_new_txt("")))){
                    Sleep(0);
                }
                //6. Enable autodisplay
                autodisplay=1;
                //7. Wait for a new keypress
                do{
                    Sleep(100);
                    if (stop_program) end();
                }while(qbs_cleanup(qbs_tmp_base,qbs_equal(qbs_inkey(),qbs_new_txt(""))));
                
                
                }else{
                if (console){
                        //screen is hidden, console is visible
                        #ifdef QBNex_WINDOWS
                            qb_print_cstr("\nPress any key to continue", 0);
                            int32 junk;
                            FlushConsoleInputBuffer(GetStdHandle(STD_INPUT_HANDLE)); //clear any stray buffer events before we run END.
                            do{ //ignore all console input
                                junk=func__getconsoleinput();
                            }while(junk!=1); //until we have a key down event
                        #else
                           cout<<"\nPress enter to continue";
                           static int32 ignore;
                           ignore=fgetc(stdin);
                       #endif
                   }
                }
            close_program=1;
            end();
            exit(0);//<-- should never happen
        }
        
        uint8 pu_dig[1024];//digits (left justified)
        int32 pu_ndig;//number of digits
        int32 pu_dp;//decimal place modifier
        //note: if dp=0, the number is an integer and can be read as is
        //      if dp=1 the number is itself*10
        //      if dp=-1 the number is itself/10
        int32 pu_neg;
        uint8 pu_buf[1024];//a buffer for preprocessing
        uint8 pu_exp_char=69; //"E"
        
        int32 print_using(qbs *f, int32 s2, qbs *dest, qbs* pu_str){
            //type: 1=numeric, 2=string
            if (new_error) return 0;
            
            static int32 x,z,z2,z3,z4,ii;
            //x  - current format string read position
            //z - used as a temp variable for various calculations and loops
            //z2  - used for various calculations involving exponent digits
            //z3 - used as a temp variable for various calculations and loops
            //z4 - number of 0s between . and digits after point
            //ii  - used as a counter for writing the output
            static uint8 c;
            static int32 stage,len,chrsleft,type,s;
            static int32 leading_plus,dollar_sign,asterisk_spaces,digits_before_point,commas;
            static int32 decimal_point,digits_after_point,trailing_plus,exponent_digits, trailing_minus;
            static int32 cant_fit,extra_sign_space,rounded,digits_and_commas_before_point,leading_zero;
            static qbs *qbs1=NULL;
            
            if (qbs1==NULL) qbs1=qbs_new(1,0);
            
            if (pu_str) type=2; else type=1;
            
            s=s2;
            len=f->len;
            
            scan:
            rounded=0;
            rounded_repass:
            
            x=s-1; //subtract one to counter pre-increment later
            
            leading_plus=0; dollar_sign=0; asterisk_spaces=0; digits_before_point=0; commas=0;
            decimal_point=0; digits_after_point=0; trailing_plus=0; exponent_digits=0; trailing_minus=0;
            digits_and_commas_before_point=0; leading_zero=0;
            stage=0;
            
            nextchar:
            x++;
            if (x<len){
                c=f->chr[x];
                chrsleft=len-x;
                
                if ((stage>=2)&&(stage<=4)){
                    
                    if (c==43){//+
                        trailing_plus=1; x++; goto numeric_spacer;
                    }
                    
                    if (c==45){//-
                        trailing_minus=1; x++; goto numeric_spacer;
                    }
                    
                }//stage>=2 & stage<=4
                
                if ((stage>=2)&&(stage<=3)){
                    
                    if (chrsleft>=5){
                        if ((c==94)&&(f->chr[x+1]==94)&&(f->chr[x+2]==94)&&(f->chr[x+3]==94)&&(f->chr[x+4]==94)){//^^^^^
                            exponent_digits=3; stage=4; x+=4; goto nextchar;
                        }
                    }//5
                    
                    if (chrsleft>=4){
                        if ((c==94)&&(f->chr[x+1]==94)&&(f->chr[x+2]==94)&&(f->chr[x+3]==94)){//^^^^
                            exponent_digits=2; stage=4; x+=3; goto nextchar;
                        }
                    }//4
                    
                }//stage>=2 & stage<=3
                
                if (stage==3){
                    
                    if (c==35){//#
                        digits_after_point++; goto nextchar;
                    }
                    
                }//stage==3
                
                if (stage==2){
                    
                    if (c==44){//,
                        commas=1; digits_before_point++; goto nextchar;
                    }
                    
                }//stage==2
                
                if (stage<=2){
                    
                    if (c==35){//#
                        digits_before_point++; stage=2; goto nextchar;
                    }
                    
                    if (c==46){//.
                        decimal_point=1; stage=3; goto nextchar;
                    }
                    
                }//stage<=2
                
                if (stage<=1){
                    
                    if (chrsleft>=3){
                        if ((c==42)&&(f->chr[x+1]==42)&&(f->chr[x+2]==36)){//**$
                            asterisk_spaces=1; digits_before_point=2; dollar_sign=1; stage=2; x+=2; goto nextchar;
                        }
                    }//3
                    
                    if (chrsleft>=2){
                        if ((c==42)&&(f->chr[x+1]==42)){//**
                            asterisk_spaces=1; digits_before_point=2; stage=2; x++; goto nextchar;
                        }
                        if ((c==36)&&(f->chr[x+1]==36)){//$$
                            dollar_sign=1; digits_before_point=1; stage=2; x++; goto nextchar;
                        }
                    }//2
                    
                }//stage 1
                
                if (stage==0){
                    
                    if (c==43){//+
                        leading_plus=1; stage=1; goto nextchar;
                    }
                    
                }//stage 0
                
                //spacer/end encountered
            }//x<len
            numeric_spacer:
            
            //valid numeric format?
            if (stage<=1) goto invalid_numeric_format;
            if ((digits_before_point==0)&&(digits_after_point==0)) goto invalid_numeric_format;
            
            if (type==0) return s; //s is the beginning of a new format but item has already been added to dest
            if (type==2){//expected string format, not numeric format
                error(13);//type mismatch
                return 0;
            }
            
            //reduce digits before point appropriatly
            extra_sign_space=0;
            if (exponent_digits){
                if ((leading_plus==0)&&(trailing_plus==0)&&(trailing_minus==0)){
                    digits_before_point--;
                    if (digits_before_point==-1){
                        digits_after_point--; digits_before_point=0;
                        if (digits_after_point==0){decimal_point=0; digits_before_point++;}
                    }
                    extra_sign_space=1;
                }
                }else{
                //the following doesn't occur if using an exponent
                if (pu_neg){
                    if ((leading_plus==0)&&(trailing_plus==0)&&(trailing_minus==0)){digits_before_point--; extra_sign_space=1;}
                }
                if (commas){
                    digits_and_commas_before_point=digits_before_point;
                    ii=digits_before_point/4;//for every 4 digits, one digit will be used up by a comma
                    digits_before_point-=ii;
                }
            }
            
            //'0'->'.0' exception (for when format doesn't allow for any digits_before_point)
            if (digits_before_point==0){//no digits allowed before decimal point
                //note: pu_ndig=256, pu_dp=-255
                if ((pu_ndig+pu_dp)==1){//1 digit exists in front of the decimal point
                    if (pu_dig[0]==48){//is it 0?
                        pu_dp--;//moves decimal point left one position
                    }//0
                }
            }
            
            //will number fit? if it can't then adjustments will be made
            cant_fit=0;
            if (exponent_digits){
                //give back extra_sign_space?
                if (extra_sign_space){
                    if (!pu_neg){
                        if (digits_before_point<=0){
                            extra_sign_space=0;
                            digits_before_point++;//will become 0 or 1
                            //force 0 in recovered digit?
                            if ((digits_before_point==1)&&(digits_after_point>0)){
                                digits_before_point--;
                                extra_sign_space=2;//2=put 0 instead of blank space
                            }
                        }
                    }
                }
                if ((digits_before_point==0)&&(digits_after_point==0)){
                    cant_fit=1;
                    digits_before_point=1;//give back removed (for extra sign space) digit
                }
                //but does the exponent fit?
                z2=pu_ndig+pu_dp-1;//calc exponent of most significant digit
                //1.0  = 0
                //10.0 = 1
                //0.1  = -1
                //calc exponent of format's most significant position
                if (digits_before_point) z3=digits_before_point-1; else z3=-1;
                z=z2-z3;//combine to calculate actual exponent which will be "printed" 
                z3=abs(z);
                z2=sprintf((char*)pu_buf,"%u",z3);//use pu_buf to convert exponent to a string
                if (z2>exponent_digits){cant_fit=1; exponent_digits=z2;}
                }else{
                z2=0;
                z=pu_ndig+pu_dp;//calc number of digits required before decimal places
                if (digits_before_point<z){
                    digits_before_point=z; cant_fit=1;
                    if (commas) digits_and_commas_before_point=digits_before_point+(digits_before_point-1)/3;
                }
            }
            
            
            
            static int32 buf_size;//buf_size is an estimation of size required
            static uint8 *cp,*buf;
            static std::vector<uint8> buf_storage;
            static int32 count;
            buf_size=256;//256 bytes to account for calc overflow (such as exponent digits)
            buf_size+=9;//%(1)+-(1)$(1)???.(1)???exponent(5)
            buf_size+=digits_before_point;
            if (commas) buf_size+=((digits_before_point/3)+2);
            buf_size+=digits_after_point;
            buf_storage.resize(buf_size);
            buf=buf_storage.data();
            cp=buf;
            count=0;//char count
            ii=0;
            
            if (asterisk_spaces) asterisk_spaces=42; else asterisk_spaces=32;//chraracter to fill blanks with
            
            if (cant_fit) {*cp++=37; count++;}//%
            
            //leading +/-
            if (leading_plus){
                if (pu_neg) *cp++=45; else *cp++=43;
                count++;
            }
            
            if (exponent_digits){
                z4=0;
                //add $?
                if (dollar_sign) {*cp++=36; count++;}//$
                //add - sign? (as sign space was not specified)
                if (extra_sign_space){
                    if (pu_neg){
                        *cp++=45;
                        }else{
                        if (extra_sign_space==2) *cp++=48; else *cp++=32;
                    }
                    count++;
                }
                //add digits left of decimal point
                for (z3=0;z3<digits_before_point;z3++){
                    if (ii<pu_ndig) *cp++=pu_dig[ii++]; else *cp++=48;
                    count++;
                }
                //add decimal point
                if (decimal_point){*cp++=46; count++;}
                //add digits right of decimal point
                for (z3=0;z3<digits_after_point;z3++){
                    if (ii<pu_ndig) *cp++=pu_dig[ii++]; else *cp++=48;
                    count++;
                    
                }
                //round last digit (requires a repass)
                if (!rounded){
                    if (ii<pu_ndig){
                        if (pu_dig[ii]>=53){//>="5" (round 5 up)
                            z=ii-1;
                            //round up pu (by adding 1 from digit at character position z)
                            //note: pu_dig is rebuilt one character to the right so highest digit can flow over into next character
                            rounded=1;
                            memmove(&pu_dig[1],&pu_dig[0],pu_ndig); pu_dig[0]=48; z++;
                            puround2:
                            pu_dig[z]++;
                            if (pu_dig[z]>57) {pu_dig[z]=48; z--; goto puround2;}
                            if (pu_dig[0]!=48){//was extra character position necessary?
                                pu_ndig++; //note: pu_dp does not require any changes  
                                }else{
                                memmove(&pu_dig[0],&pu_dig[1],pu_ndig);
                            }
                            goto rounded_repass;
                        }
                    }
                }
                //add exponent...
                *cp++=pu_exp_char; count++; //add exponent D/E/F (set and restored by calling function as necessary)
                if (z>=0) {*cp++=43; count++;} else {*cp++=45; count++;} //+/- exponent's sign
                //add exponent's leading 0s (if any)
                for (z3=0;z3<(exponent_digits-z2);z3++){
                    *cp++=48; count++;
                }
                //add exponent's value
                for (z3=0;z3<z2;z3++){
                    *cp++=pu_buf[z3]; count++;
                }
                }else{
                //"print" everything before the point
                //calculate digit spaces before the point in number
                if (!commas) digits_and_commas_before_point=digits_before_point;
                z=pu_ndig+pu_dp;//num of character whole portion of number requires
                z4=0; if (z<0) z4=-z;//number of 0s between . and digits after point
                if (commas) z=z+(z-1)/3;//including appropriate amount of commas
                if (z<0) z=0;
                z2=digits_and_commas_before_point-z;
                if ((z==0)&&(z2>0)){leading_zero=1; z2--;}//change .1 to 0.1 if possible
                for (z3=0;z3<z2;z3++){*cp++=asterisk_spaces; count++;}
                //add - sign? (as sign space was not specified)
                if (extra_sign_space){*cp++=45; count++;}
                //add $?
                if (dollar_sign){*cp++=36; count++;}//$ 
                //leading 0?
                if (leading_zero){*cp++=48; count++;}//0
                //add digits left of decimal point
                for (z3=0;z3<z;z3++){
                    if ((commas!=0)&&(((z-z3)&3)==0)){
                        *cp++=44;
                        }else{
                        if (ii<pu_ndig) *cp++=pu_dig[ii++]; else *cp++=48;
                    }
                    count++;
                }
                //add decimal point
                if (decimal_point){*cp++=46; count++;}
                //add digits right of decimal point
                for (z3=0;z3<digits_after_point;z3++){
                    if (z4){
                        z4--;
                        *cp++=48;
                        }else{
                        if (ii<pu_ndig) *cp++=pu_dig[ii++]; else *cp++=48;
                    }
                    count++;
                }
                //round last digit (requires a repass)
                if (!rounded){
                    if (ii<pu_ndig){
                        if (pu_dig[ii]>=53){//>="5" (round 5 up)
                            z=ii-1;
                            //round up pu (by adding 1 from digit at character position z)
                            //note: pu_dig is rebuilt one character to the right so highest digit can flow over into next character
                            rounded=1;  
                            memmove(&pu_dig[1],&pu_dig[0],pu_ndig); pu_dig[0]=48; z++;
                            puround1: 
                            pu_dig[z]++;
                            if (pu_dig[z]>57) {pu_dig[z]=48; z--; goto puround1;}
                            if (pu_dig[0]!=48){//was extra character position necessary?
                                pu_ndig++; //note: pu_dp does not require any changes  
                                }else{
                                memmove(&pu_dig[0],&pu_dig[1],pu_ndig);  
                            }
                            goto rounded_repass;
                        } 
                    }
                }
            }//exponent_digits
            
            //add trailing sign?
            //trailing +/-
            if (trailing_plus){
                if (pu_neg) *cp++=45; else *cp++=43;
                count++;
            }
            //trailing -
            if (trailing_minus){
                if (pu_neg) *cp++=45; else *cp++=32;
                count++;
            }
            
            qbs_set(dest,qbs_add(dest,qbs_new_txt_len((char*)buf,count)));
            
            s=x;
            type=0;//passed type added
            if (s>=len) return 0;//end of format line encountered and passed item added
            goto scan;
            
            invalid_numeric_format:
            //string format
            static int32 string_size;
            
            x=s;
            if (x<len){
                c=f->chr[x];
                string_size=0;//invalid
                if (c==38) string_size=-1; //"&" (all of string)
                if (c==33) string_size=1; //"!" (first character only)
                if (c==92){ //"\" first n characters
                    z=1;
                    x++;
                    get_str_fmt:
                    if (x>=len) goto invalid_string_format;
                    c=f->chr[x];
                    z++;
                    if (c==32){x++; goto get_str_fmt;}
                    if (c!=92) goto invalid_string_format;
                    string_size=z;
                }//c==47
                if (string_size){
                    if (type==0) return s; //s is the beginning of a new format but item has already been added to dest
                    if (type==1){//expected numeric format, not string format
                        error(13);//type mismatch
                        return 0;
                    }
                    if (string_size!=-1){
                        s+=string_size;
                        for (z=0;z<string_size;z++){
                            if (z<pu_str->len) qbs1->chr[0]=pu_str->chr[z]; else qbs1->chr[0]=32;
                            qbs_set(dest,qbs_add(dest,qbs1));
                        }//z 
                        }else{
                        qbs_set(dest,qbs_add(dest,pu_str));
                        s++;
                    }
                    type=0;//passed type added
                    if (s>=len) return 0;//end of format line encountered and passed item added
                    goto scan;
                }//string_size
            }//x<len
            invalid_string_format:
            
            //add literal?
            if ((f->chr[s]==95)&&(s<(len-1))){//trailing single _ in format is treated as a literal _
                s++;
            }
            //add non-format character
            qbs1->chr[0]=f->chr[s]; qbs_set(dest,qbs_add(dest,qbs1));
            
            s++;
            if (s>=len){
                s=0;
                if (type==0) return s;//end of format line encountered and passed item added
                //illegal format? (format has been passed from start (s2=0) to end and has no numeric/string identifiers
                if (s2==0){
                    error(5);//illegal function call
                    return 0;
                }
            }
            goto scan;
            
            return 0;
        }
        
        int32 print_using_integer64(qbs* format, int64 value, int32 start, qbs *output){
            if (new_error) return 0;
            #ifdef QBNex_WINDOWS
                pu_ndig=sprintf((char*)pu_buf,"% I64i",value);
                #else
                pu_ndig=sprintf((char*)pu_buf,"% lli",value);
            #endif
            if (pu_buf[0]==45) pu_neg=1; else pu_neg=0;
            pu_ndig--;//remove sign
            memcpy(pu_dig,&pu_buf[1],pu_ndig);
            pu_dp=0;
            start=print_using(format,start,output,NULL);
            return start;
        }
        
        int32 print_using_uinteger64(qbs* format, uint64 value, int32 start, qbs *output){
            if (new_error) return 0;
            #ifdef QBNex_WINDOWS
                pu_ndig=sprintf((char*)pu_dig,"%I64u",value);
                #else
                pu_ndig=sprintf((char*)pu_dig,"%llu",value);
            #endif
            pu_neg=0;
            pu_dp=0;
            start=print_using(format,start,output,NULL);
            return start;
        }
        
        int32 print_using_single(qbs* format, float value, int32 start, qbs *output){
            if (new_error) return 0;
            static int32 i,len,neg_exp;
            static uint8 c;
            static int64 exp;
            len=sprintf((char*)&pu_buf,"% .255E",value);//256 character limit ([1].[255])
            pu_dp=0;
            pu_ndig=0;
            //1. sign
            if (pu_buf[0]==45) pu_neg=1; else pu_neg=0;
            i=1;
            //2. digits before decimal place
            getdigits:
            if (i>=len){error(5); return 0;}
            c=pu_buf[i];
            if ((c>=48)&&(c<=57)){
                pu_dig[pu_ndig++]=c;
                i++;
                goto getdigits;
            }
            //3. decimal place
            if (c!=46){error(5); return 0;}//expected decimal point
            i++;
            //4. digits after decimal place
            getdigits2:
            if (i>=len){error(5); return 0;}
            c=pu_buf[i];
            if ((c>=48)&&(c<=57)){
                pu_dig[pu_ndig++]=c;
                pu_dp--;
                i++;
                goto getdigits2;
            }
            //assume random character signifying an exponent
            i++;
            //optional exponent sign
            neg_exp=0;
            if (i>=len){error(5); return 0;}
            c=pu_buf[i];
            if (c==45){neg_exp=1; i++;}//-
            if (c==43) i++;//+
            //assume remaining characters are an exponent
            exp=0;
            getdigits3:
            if (i<len){
                c=pu_buf[i];
                if ((c<48)||(c>57)){error(5); return 0;}
                exp=exp*10;
                exp=exp+c-48;
                i++;
                goto getdigits3;
            }
            if (neg_exp) exp=-exp;
            pu_dp+=exp;
            start=print_using(format,start,output,NULL);
            return start;
        }
        
        int32 print_using_double(qbs* format, double value, int32 start, qbs *output){
            if (new_error) return 0;
            static int32 i,len,neg_exp;
            static uint8 c;
            static int64 exp;
            len=sprintf((char*)&pu_buf,"% .255E",value);//256 character limit ([1].[255])
            pu_dp=0;
            pu_ndig=0;
            //1. sign
            if (pu_buf[0]==45) pu_neg=1; else pu_neg=0;
            i=1;
            //2. digits before decimal place
            getdigits:
            if (i>=len){error(5); return 0;}
            c=pu_buf[i];
            if ((c>=48)&&(c<=57)){
                pu_dig[pu_ndig++]=c;
                i++;
                goto getdigits;
            }
            //3. decimal place
            if (c!=46){error(5); return 0;}//expected decimal point
            i++;
            //4. digits after decimal place
            getdigits2:
            if (i>=len){error(5); return 0;}
            c=pu_buf[i];
            if ((c>=48)&&(c<=57)){
                pu_dig[pu_ndig++]=c;
                pu_dp--;
                i++;
                goto getdigits2;
            }
            //assume random character signifying an exponent
            i++;
            //optional exponent sign
            neg_exp=0;
            if (i>=len){error(5); return 0;}
            c=pu_buf[i];
            if (c==45){neg_exp=1; i++;}//-
            if (c==43) i++;//+
            //assume remaining characters are an exponent
            exp=0;
            getdigits3:
            if (i<len){
                c=pu_buf[i];
                if ((c<48)||(c>57)){error(5); return 0;}
                exp=exp*10;
                exp=exp+c-48;
                i++;
                goto getdigits3;
            }
            if (neg_exp) exp=-exp;
            pu_dp+=exp;
            pu_exp_char=68; //"D"
            start=print_using(format,start,output,NULL);
            pu_exp_char=69; //"E"
            return start;
        }
        
        //WARNING: DUE TO MINGW NOT SUPPORTING PRINTF long double, VALUES ARE CONVERTED TO A DOUBLE
        //         BUT PRINTED AS IF THEY WERE A long double
        int32 print_using_float(qbs* format, long double value, int32 start, qbs *output){
            if (new_error) return 0;
            static int32 i,len,neg_exp;
            static uint8 c;
            static int64 exp;
            //len=sprintf((char*)&pu_buf,"% .255E",value);//256 character limit ([1].[255])
            #ifdef QBNex_MINGW
                len=__mingw_sprintf((char*)&pu_buf,"% .255Lf",value);//256 character limit ([1].[255])
                #else 
                len=sprintf((char*)&pu_buf,"% .255Lf",value);//256 character limit ([1].[255])
            #endif
            
            //qbs_print(qbs_new_txt((char*)&pu_buf),1);
            
            
            pu_dp=0;
            pu_ndig=0;
            //1. sign
            if (pu_buf[0]==45) pu_neg=1; else pu_neg=0;
            i=1;
            //2. digits before decimal place
            getdigits:
            if (i>=len){error(5); return 0;}
            c=pu_buf[i];
            if ((c>=48)&&(c<=57)){
                pu_dig[pu_ndig++]=c;
                i++;
                goto getdigits;
            }
            //3. decimal place
            if (c!=46){error(5); return 0;}//expected decimal point
            i++;
            //4. digits after decimal place
            getdigits2:
            if (i>=len){
                //no exponent information has been provided
                neg_exp=0;
                exp=0;
                goto no_exponent_provided;
                //error(5); return 0;
            }
            c=pu_buf[i];
            if ((c>=48)&&(c<=57)){
                pu_dig[pu_ndig++]=c;
                pu_dp--;
                i++;
                goto getdigits2;
            }
            //assume random character signifying an exponent
            i++;
            //optional exponent sign
            neg_exp=0;
            if (i>=len){error(5); return 0;}
            c=pu_buf[i];
            if (c==45){neg_exp=1; i++;}//-
            if (c==43) i++;//+
            //assume remaining characters are an exponent
            exp=0;
            getdigits3:
            if (i<len){
                c=pu_buf[i];
                if ((c<48)||(c>57)){error(5); return 0;}
                exp=exp*10;
                exp=exp+c-48;
                i++;
                goto getdigits3;
            }
            if (neg_exp) exp=-exp;
            pu_dp+=exp;
            no_exponent_provided:
            pu_exp_char=70; //"F"
            start=print_using(format,start,output,NULL);
            pu_exp_char=69; //"E"
            return start;
        }
        
        void sub_run_init(){
            //Reset ON KEY trapping
            //note: KEY bar F-key bindings are not affected
            static int32 i;
            for (i=1;i<=31;i++){
                onkey[i].id=0;
                onkey[i].active=0;
                onkey[i].state=0;
            }
            onkey_inprogress=0;
            //note: if already in screen 0:80x25, screen pages are left intact
            //set screen mode to 0 (80x25)
            qbg_screen(0,NULL,0,0,NULL,1|4|8);
            //make sure WIDTH is 80x25
            qbsub_width(NULL,80,25,0,0,1|2);
            //restore view print
            qbg_sub_view_print(NULL,NULL,0);
            //restore palette
            restorepalette(write_page);
            //restore default colors
            write_page->background_color=0;
            write_page->color=7;
            //note: cursor state does not appear to be reset by the RUN command
            //im->cursor_show=0; im->cursor_firstvalue=4; im->cursor_lastvalue=4;
            //Reset RND & RANDOMIZE state
            qb_rnd_reset();
            //clear keyboard buffers
            sub__keyclear(NULL,0);
        }
        
        
        
        void sub_run(qbs* f){
            if (new_error) return;
            
            //run program
            static qbs *str=NULL;
            if (str==NULL) str=qbs_new(0,0);
            static qbs *strz=NULL;
            if (!strz) strz=qbs_new(0,0);
            
            qbs_set(str,f);
            fixdir(str);
            
            #ifdef QBNex_WINDOWS
                
                qbs_set(strz,qbs_add(str,qbs_new_txt_len("\0",1)));
                if (WinExec((char *)strz->chr,SW_SHOWDEFAULT)>31){
                    goto run_exit;
                    }else{
                    //0-out of resources/memory
                    //ERROR_BAD_FORMAT
                    //ERROR_FILE_NOT_FOUND
                    //ERROR_PATH_NOT_FOUND
                    error(53); return;//file not found
                }
                
                #else
                qbs_set(strz,qbs_add(str,qbs_new_txt_len("\0",1)));
                system((char*)strz->chr);
                //success?
                goto run_exit;
                
            #endif
            
            //exit this program
            run_exit:
            close_program=1;
            end();
            exit(99);//<--this line should never actually be executed
            
        }
        
        #ifdef DEPENDENCY_ICON
            void sub__icon(int32 handle_icon, int32 handle_window_icon, int32 passed){
                
                if (new_error) return;
                #ifndef DEPENDENCY_CONSOLE_ONLY
                if (!(passed&2)) handle_window_icon=handle_icon;
                if (!(passed&1)){
                    handle_icon=image_qbicon32_handle;
                    handle_window_icon=image_qbicon16_handle;
                }
                
                static int32 i,i2,ii,w,h;
                static uint32 *o,*o2;
                static int32 x,y,n,c,i3,c2;
                
                //validation
                for (ii=1;ii<=2;ii++){
                    if (ii==1) i=handle_icon;
                    if (ii==2) i=handle_window_icon;
                    if (!resolve_page_or_image_index(i,i)) return;
                    if (img[i].text){error(5); return;}
                    if (ii==1) handle_icon=i;
                    if (ii==2) handle_window_icon=i;
                }
                
                if (!screen_hide){
                    while (!window_exists){Sleep(100);}
                    
                    #ifdef QBNex_WINDOWS
                        while (!window_handle){Sleep(100);}
                        
                        static HANDLE ExeIcon;
                        static HANDLE ExeIcon16;
                        
                        //Attempt to load the first icon embedded in the .exe
                        if (!ExeIcon) ExeIcon = LoadImage(GetModuleHandle(NULL), MAKEINTRESOURCE(0), IMAGE_ICON,32, 32, 0);
                        if (!ExeIcon16) ExeIcon16 = LoadImage(GetModuleHandle(NULL), MAKEINTRESOURCE(0), IMAGE_ICON,16, 16, 0);
                        
                        //If we have an embedded icon, we'll use it instead of QBNex's default
                        if (!(passed&1) && (ExeIcon)) {
                            SendMessage( window_handle, WM_SETICON, ICON_BIG,   (LPARAM)ExeIcon );
                            
                            if (ExeIcon16) {
                            SendMessage( window_handle, WM_SETICON, ICON_SMALL, (LPARAM)ExeIcon16 ); }
                            else {
                            SendMessage( window_handle, WM_SETICON, ICON_SMALL, (LPARAM)ExeIcon ); }
                            return;
                        }
                        for (ii=1;ii<=2;ii++){
                            
                            if (ii==1){i=handle_icon; w=GetSystemMetrics(SM_CXICON); h=GetSystemMetrics(SM_CYICON);}
                            if (ii==2){i=handle_window_icon; w=GetSystemMetrics(SM_CXSMICON); h=GetSystemMetrics(SM_CYSMICON);}
                            
                            //source[http://support.microsoft.com/kb/318876]
                            ICONINFO iinfo;
                            HDC hdc;
                            BITMAPV5HEADER bi;
                            HBITMAP hBitmap, hOldBitmap;
                            void *lpBits;
                            HCURSOR hAlphaCursor = NULL;
                            ZeroMemory(&bi,sizeof(BITMAPV5HEADER));
                            bi.bV5Size           = sizeof(BITMAPV5HEADER);
                            bi.bV5Width           = w;
                            bi.bV5Height          = h;
                            bi.bV5Planes = 1;
                            bi.bV5BitCount = 32;
                            bi.bV5Compression = BI_BITFIELDS;
                            // The following mask specification specifies a supported 32 BPP
                            // alpha format for Windows XP.
                            bi.bV5RedMask   =  0x00FF0000;
                            bi.bV5GreenMask =  0x0000FF00;
                            bi.bV5BlueMask  =  0x000000FF;
                            bi.bV5AlphaMask =  0xFF000000; 
                            hdc = GetDC(NULL);
                            // Create the DIB section with an alpha channel.
                            hBitmap = CreateDIBSection(hdc, (BITMAPINFO *)&bi, DIB_RGB_COLORS, 
                            (void **)&lpBits, NULL, (DWORD)0);
                            ReleaseDC(NULL,hdc);
                            
                            i2=func__newimage(w,h,32,1);
                            sub__dontblend(i2,1);
                            sub__putimage(NULL,NULL,NULL,NULL,-i,i2,NULL,NULL,NULL,NULL,8+32);
                            
                            o=img[-i2].offset32;
                            o2=(uint32*)lpBits;
                            for (y=0;y<h;y++){
                                for (x=0;x<w;x++){
                                    c=o[(h-1-y)*w+x];
                                    o2[y*w+x]=c;
                                }}
                                
                                sub__freeimage(i2,1);
                                
                                // Create an empty mask bitmap.
                                HBITMAP hMonoBitmap = CreateBitmap(w,h,1,1,NULL);
                                
                                iinfo.fIcon = TRUE;  // Change fIcon to TRUE to create an alpha icon
                                iinfo.xHotspot = 0;
                                iinfo.yHotspot = 0;
                                iinfo.hbmMask = hMonoBitmap;
                                iinfo.hbmColor = hBitmap;
                                // Create the alpha cursor with the alpha DIB section.
                                hAlphaCursor = CreateIconIndirect(&iinfo);
                                
                                DeleteObject(hBitmap);
                                DeleteObject(hMonoBitmap);
                                
                                if (ii==1) SendMessage( window_handle, WM_SETICON, ICON_BIG,   (LPARAM)hAlphaCursor );
                                if (ii==2) SendMessage( window_handle, WM_SETICON, ICON_SMALL, (LPARAM)hAlphaCursor );
                                
                        }//ii
                        
                    #endif //QBNex_WINDOWS
                }//!screen_hide
                #endif //DEPENDENCY_CONSOLE_ONLY
            }//sub__icon
        #endif //DEPENDENCY_ICON

        int32 func_screenwidth () {
            #ifdef QBNex_GUI
                int32 w,h; qb_desktop_size(&w,&h); return w;
            #else
                #ifdef QBNex_WINDOWS
                    return GetSystemMetrics(0);
                #else
                    return 0;
                #endif
            #endif
        }

        int32 func_screenheight () {
            #ifdef QBNex_GUI
                int32 w,h; qb_desktop_size(&w,&h); return h;
            #else
                #ifdef QBNex_WINDOWS
                    return GetSystemMetrics(1);
                #else
                    return 0;
                #endif
            #endif
        }

        void sub_screenicon () {
            #ifdef QBNex_GUI
                while (!window_exists){Sleep(100);}
                qb_window_minimize();
            #endif
        }

        int32 func_windowexists () {
            #ifdef QBNex_GUI
                return -window_exists;
            #else
                return -1;
            #endif
        }

        int32 func_screenicon () {
            #ifdef QBNex_GUI
                while (!window_exists){Sleep(100);}
                extern int32 screen_hide;
                if (screen_hide) {error(5); return 0;}
                return -qb_window_minimized();
            #endif
            return 0;
        }

        int32 func__autodisplay () {
            if (autodisplay) {return -1;}
            return 0;
        }
        void sub__autodisplay(){
            autodisplay=1;
        }
        
        void sub__display(){
            qb_fb_capture(display_page,0);//headless framebuffer capture (QBNEX_FB_CAPTURE)
            
            if (screen_hide) return;
            
            //disable autodisplay (if enabled)
            if (autodisplay){
                autodisplay=-1;//toggle request
                while(autodisplay) Sleep(1);
                return;//note: autodisplay is set to 0 after display() has been called so a second call to display() is unnecessary
            }
            display();
        }
        
        //DRAW and its number parser: Rust (qb-runtime gfx/draw.rs); VARPTR$ arguments point into cmem
        void sub_draw(qbs* s){
            if (new_error) return;
            if (int32 e=qb_gfx_draw(write_page,s->chr,s->len,cmem)) error(e);
        }

        #ifdef QBNex_WINDOWS
            #define envp _environ
        #else
            extern char **environ;
            #define envp environ
        #endif

        //ENVIRON$, ENVIRON, _ENVIRONCOUNT: implemented in Rust (crates/qb-runtime/src/env.rs)
        int32 func__environcount() {
            return qb_environ_count(envp);
        }

        qbs *func_environ(qbs *name) {
            const uint8 *out; int32 len=0;
            qb_environ_get(name->chr,name->len,&out,&len);
            qbs *tqbs=qbs_new(len,1);
            memcpy(tqbs->chr,out,len);
            return tqbs;
        }

        qbs *func_environ(int32 number) {
            const uint8 *out; int32 len=0;
            int32 e=qb_environ_index(envp,number,&out,&len);
            qbs *tqbs=qbs_new(len,1);
            memcpy(tqbs->chr,out,len);
            if (e) error(e);
            return tqbs;
        }

        void sub_environ(qbs *str) {
            int32 e=qb_environ_set(str->chr,str->len); if (e) error(e);
        }
        
        #ifdef QBNex_WINDOWS
            void showvalue(__int64 v){
                static qbs* s=NULL;
                if (s==NULL) s=qbs_new(0,0);
                qbs_set(s,qbs_str(v));
                MessageBox2(NULL,(char*)s->chr,"showvalue",MB_OK|MB_SYSTEMMODAL);
            }
        #endif

        
        // TCP/IP sockets, the info-string parser and WHATISMYIP live in the Rust
        // runtime (crates/qb-runtime/src/net.rs). The handle tables below stay here
        // because file I/O (GET/PUT/CLOSE) shares them.

        struct connection_struct{
            int8 in_use;//0=not being used, 1=in use
            int8 protocol;//1=TCP/IP
            int8 type;//1=client, 2=host(listening), 3=host's connection from a client
            ptrszint stream;
            ptrszint handle;
            void *connection;//qb_tcp*
            //---------------------------------
            int32 port;
        };
        list *connection_handles=NULL;

        void stream_out(stream_struct *st,void *offset,ptrszint bytes){
            if (st->type==1){//Network
                static connection_struct *co; co=(connection_struct*)st->index;
                if ((co->type==1)||(co->type==3)){//client or host's connection from a client
                    if (co->protocol==1) qb_tcp_send((qb_tcp*)co->connection,(const uint8_t*)offset,bytes);
                }
            }//Network
        }//stream_out

        void stream_update(stream_struct *stream){
            #ifdef DEPENDENCY_SOCKETS
                //assume tcp
                connection_struct *connection=(connection_struct*)(stream->index);
                qb_tcp *tcp=(qb_tcp*)(connection->connection);
                if (!stream->in_limit){
                    stream->in_size=0;
                    resize_stream_input_buffer(stream,1024);
                }
                expand_and_retry:
                //expand buffer if 'in' stream is full (the request to qb_tcp_recv is never 0 bytes)
                if (stream->in_size==stream->in_limit){
                    resize_stream_input_buffer(stream,stream->in_limit*2);
                }
                ptrszint bytes=qb_tcp_recv(tcp,stream->in+stream->in_size,stream->in_limit-stream->in_size);
                if (bytes>0){
                    stream->in_size+=bytes;
                    if (stream->in_size==stream->in_limit) goto expand_and_retry;
                }
            #endif
        }

        void connection_close(ptrszint i){
            //Note: 'i' is a positive integer 1 or greater
            //      'i' must be a valid handle
            static special_handle_struct *sh; sh=(special_handle_struct*)list_get(special_handles,i);

            if (sh->type==2){//host listener
                static connection_struct *cs; cs=(connection_struct*)sh->index;
                if (cs->protocol==1) qb_tcp_close((qb_tcp*)cs->connection);
                list_remove(connection_handles,list_get_index(connection_handles,cs));
                list_remove(special_handles,list_get_index(special_handles,sh));
                return;
            }//host listener

            //client or connection to host
            if (sh->type==1){//stream
                static stream_struct *ss; ss=(stream_struct*)sh->index;
                if (ss->type==1){//network
                    static connection_struct *cs; cs=(connection_struct*)ss->index;
                    if (cs->protocol==1) qb_tcp_close((qb_tcp*)cs->connection);
                    list_remove(connection_handles,list_get_index(connection_handles,cs));
                    stream_free(ss);
                    list_remove(special_handles,list_get_index(special_handles,sh));
                    return;
                }//network
            }//stream

        }

        static int32 connection_register(void *connection,int32 type,int32 port){
            //type: 1=client, 2=host(listening), 3=host's connection from a client
            static int32 my_handle; my_handle=list_add(special_handles);
            static special_handle_struct *my_handle_struct; my_handle_struct=(special_handle_struct*)list_get(special_handles,my_handle);
            static int32 my_connection; my_connection=list_add(connection_handles);
            static connection_struct *my_connection_struct; my_connection_struct=(connection_struct*)list_get(connection_handles,my_connection);
            my_connection_struct->protocol=1;//tcp/ip
            my_connection_struct->type=type;
            my_connection_struct->connection=connection;
            my_connection_struct->port=port;
            if (type==2){
                my_handle_struct->type=2;//host listener
                my_handle_struct->index=(ptrszint)my_connection_struct;
                return my_handle;
            }
            static int32 my_stream; my_stream=list_add(stream_handles);
            static stream_struct *my_stream_struct; my_stream_struct=(stream_struct*)list_get(stream_handles,my_stream);
            my_handle_struct->type=1;//stream
            my_handle_struct->index=(ptrszint)my_stream_struct;
            my_stream_struct->type=1;//network
            my_stream_struct->index=(ptrszint)my_connection_struct;
            //init stream
            my_stream_struct->in=NULL; my_stream_struct->in_size=0; my_stream_struct->in_limit=0;
            return my_handle;
        }

        int32 connection_new(int32 method, qbs *info, int32 value){
            //method: 0=_OPENCLIENT [info=~"TCP/IP:12345:23.96.32.123], value=NULL"
            //        1=_OPENHOST [info=~"TCP/IP:12345", value=NULL]
            //        2=_OPENCONNECTION [info=NULL, value=host's handle]
            //returns: -1=invalid arguments passed
            //          0=failed to open
            //         >0=handle of successfully opened connection
            void *connection;
            if ((method==0)||(method==1)){
                int32 port,host_off,host_len,val_error=0;
                int32 r=qb_net_parse_info(info->chr,info->len,method,vwatch==-1,&port,&host_off,&host_len,&val_error);
                if (val_error) error(val_error);//legacy: VAL raised it and the open went on
                if (r) return -1;
                if (method==0){//_OPENCLIENT
                    connection=qb_tcp_client_open(info->chr+host_off,host_len,port);
                    if (!connection) return 0;
                    int32 my_handle=connection_register(connection,1,port);
                    if (vwatch==-1) vwatch=my_handle;
                    return my_handle;
                }
                connection=qb_tcp_host_open(port);//_OPENHOST
                if (!connection) return 0;
                return connection_register(connection,2,port);
            }
            //_OPENCONNECTION
            static special_handle_struct *sh; sh=(special_handle_struct*)list_get(special_handles,value); if (!sh) return -1;
            if (sh->type!=2) return -1;//listening host?
            static connection_struct *co; co=(connection_struct*)sh->index;
            connection=qb_tcp_accept((qb_tcp*)co->connection);
            if (!connection) return 0;
            return connection_register(connection,3,0);
        }//connection_new

        int32 func__openclient(qbs* info){
            if (new_error) return 0;
            static int32 i;
            i=connection_new(0,info,NULL);
            if (i==-1){error(5); return 0;}
            if (i==0) return 0;
            return -1-i;
        }

        int32 func__openhost(qbs* info){
            if (new_error) return 0;
            static int32 i;
            i=connection_new(1,info,NULL);
            if (i==-1){error(5); return 0;}
            if (i==0) return 0;
            return -1-i;
        }

        int32 func__openconnection(int32 i){
            if (new_error) return 0;
            i=-(i+1);
            i=connection_new(2,NULL,i);
            if (i==-1){error(258); return 0;}//invalid handle
            if (i==0) return 0;//no new connections
            return -1-i;
        }

        static qbs *tcp_address(void *connection,int32 type){
            uint8 buf[512];
            int32 n=qb_tcp_address((qb_tcp*)connection,type,buf,sizeof(buf));
            qbs *tqbs=qbs_new(n,1);
            if (n<=(int32)sizeof(buf)) memcpy(tqbs->chr,buf,n); else qb_tcp_address((qb_tcp*)connection,type,tqbs->chr,n);
            return tqbs;
        }

        qbs *func__connectionaddress(int32 i){
            static int32 x;
            if (new_error) goto error;
            if (i<0){
                x=-(i+1);
                static special_handle_struct *sh; sh=(special_handle_struct*)list_get(special_handles,x); if (!sh){error(52); goto error;}
                if (sh->type==2){//host listener
                    static connection_struct *cs; cs=(connection_struct*)sh->index;
                    if (cs->protocol==1) return tcp_address(cs->connection,2);
                }//host listener
                //client or connection to host
                if (sh->type==1){//stream
                    static stream_struct *ss; ss=(stream_struct*)sh->index;
                    if (ss->type==1){//network
                        static connection_struct *cs; cs=(connection_struct*)ss->index;
                        if (cs->protocol==1){//TCP/IP
                            if (cs->type==1||cs->type==3) return tcp_address(cs->connection,cs->type);
                        }
                    }//network
                }//stream
            }//i<0
            error(52);
            error:
            return qbs_new(0,1);
        }

        int32 func__connected(int32 i){
            if (new_error) return 0;
            if (i<0){
                static int32 x;
                x=-(i+1);
                static special_handle_struct *sh; sh=(special_handle_struct*)list_get(special_handles,x); if (!sh) goto error;
                if (sh->type==2){//host listener
                    static connection_struct *cs; cs=(connection_struct*)sh->index;
                    if (cs->protocol==1) return -1;
                }//host listener
                //client or connection to host
                if (sh->type==1){//stream
                    static stream_struct *ss; ss=(stream_struct*)sh->index;
                    if (ss->type==1){//network
                        static connection_struct *cs; cs=(connection_struct*)ss->index;
                        if (cs->protocol==1) return qb_tcp_connected((qb_tcp*)cs->connection);
                    }//network
                }//stream
            }//i<0
            error: error(52); return 0;
        }
        
        
        int32 func__exit(){
            exit_blocked=1;
            return qb_take_exit_request();
        }
        
        
        
        
        
        
        
        
        // Clipboard text and images live in the Rust runtime
        // (crates/qb-runtime/src/clipboard.rs): CF_TEXT/CF_BITMAP on Windows,
        // arboard on Linux and macOS.

        void sub__clipboard(qbs *text){
            qb_clipboard_set_text(text->chr,text->len);
        }

        #ifdef DEPENDENCY_SCREENIMAGE
            void sub__clipboardimage(int32 src) {
                if (new_error) return;
                static int32 i,i2,w,h;
                //validation
                i=src;
                if (!resolve_page_or_image_index(i,i)) return;
                if (img[i].text){error(5); return;}
                //end of validation
                w=img[i].width;
                h=img[i].height;
                //Transfer the source image to a new 32-bit image to avoid incompatible formats
                i2=func__newimage(w,h,32,1);
                sub__putimage(NULL,NULL,NULL,NULL,-i,i2,NULL,NULL,NULL,NULL,8+32);
                if (i2<-1) qb_clipboard_set_image(img[-i2].offset32,w,h);
                sub__freeimage(i2,1);
            }
        #endif

        #ifdef DEPENDENCY_SCREENIMAGE
            int32 func__clipboardimage(){
                if (new_error) return -1;
                int32 w,h;
                uint32 *pixels=qb_clipboard_get_image(&w,&h);
                if (!pixels) return -1;
                static int32 i;
                i=func__newimage(w,h,32,1);
                if (i<-1) memcpy(img[-i].offset32,pixels,(size_t)w*h*4);
                qb_free_pixels(pixels,(int64)w*h);
                return i;
            }
        #endif

        qbs *func__clipboard(){
            int64 len;
            uint8 *text=qb_clipboard_get_text(&len);
            qbs *tqbs=qbs_new(len,1);
            if (text){
                memcpy(tqbs->chr,text,len);
                qb_free_bytes(text,len);
            }
            return tqbs;
        }
        int32 display_called=0;
        void display_now(){
            if (autodisplay){
                display_called=0;
                while(!display_called) Sleep(1);
                }else{
                display();
            }
        }
        
        void sub__fullscreen(int32 method,int32 passed){
            //ref: "[{_OFF|_STRETCH|_SQUAREPIXELS|OFF}][, _SMOOTH]"
            //          1      2           3        4         1
            int32 x;
            if (method==0) x=1;
            if (method==1||method==4) x=0;
            if (method==2) x=1;
            if (method==3) x=2;
            if (passed&1) fullscreen_smooth=1; else fullscreen_smooth=0;
            if (full_screen!=x) full_screen_set=x;
            force_display_update=1;
        }

        void sub__allowfullscreen(int32 method,int32 smooth){
            //ref: "[{_STRETCH|_SQUAREPIXELS|_OFF|_ALL|OFF}][, _SMOOTH|_OFF|_ALL|OFF]"
            //            1          2         3    4   5         1      2    3   4
            
            fullscreen_allowedmode=method;
            if (method==3||method==5)
            fullscreen_allowedmode=-1;
            if (method==4||method==NULL)
            fullscreen_allowedmode=0;
            
            fullscreen_allowedsmooth=smooth;
            if (smooth==2||smooth==4)
            fullscreen_allowedsmooth=-1;
            if (smooth==3||smooth==NULL)
            fullscreen_allowedsmooth=0;
        }
        
        int32 func__fullscreen(){
            static int32 x;
            x=full_screen_set;
            if (x!=-1) return x;
            return full_screen;
        }
        
        int32 func__fullscreensmooth(){
            return -fullscreen_smooth;
        }
        
        void chain_restorescreenstate(int32 i){
            static int32 i32,i32b,i32c,x,x2;
            generic_get(i,-1,(uint8*)&i32,4);
            
            if (i32==256){
                generic_get(i,-1,(uint8*)&i32,4);
                if (i32!=0) qbg_screen(i32,0,0,0,0,1);
                generic_get(i,-1,(uint8*)&i32,4);
                if (i32==258){
                    generic_get(i,-1,(uint8*)&i32,4); i32b=i32;
                    generic_get(i,-1,(uint8*)&i32,4);
                    qbsub_width(0,i32b,i32,0,0,1+2);
                    generic_get(i,-1,(uint8*)&i32,4);
                }
            }
            
            if (i32==257){
                generic_get(i,-1,(uint8*)&i32,4); i32c=i32;
                generic_get(i,-1,(uint8*)&i32,4); i32b=i32;
                generic_get(i,-1,(uint8*)&i32,4);
                qbg_screen(func__newimage(i32b,i32,i32c,1),0,0,0,0,1);
                generic_get(i,-1,(uint8*)&i32,4);
            }
            
            if (i32==259){
                generic_get(i,-1,(uint8*)&i32,4);
                sub__font(i32,0,0);
                generic_get(i,-1,(uint8*)&i32,4);
            }
            
            static img_struct *ix;
            static img_struct imgs;
            
            while(i32==260){
                generic_get(i,-1,(uint8*)&i32,4); x=i32;
                qbg_screen(0,0,x,0,0,4+8);//switch to page (allocates the page)
                ix=&img[page[x]];
                generic_get(i,-1,ix->offset,ix->width*ix->height*ix->bytes_per_pixel);
                imgs=*ix;
                generic_get(i,-1,(uint8*)ix,sizeof(img_struct));
                //revert specific data
                if (ix->font>=32) ix->font=imgs.font;
                ix->offset=imgs.offset;
                ix->pal=imgs.pal;
                generic_get(i,-1,(uint8*)&i32,4);
            }
            
            if (i32==261){
                generic_get(i,-1,(uint8*)&i32,4); i32b=i32;
                generic_get(i,-1,(uint8*)&i32,4);
                qbg_screen(0,0,i32b,i32,0,4+8);//switch to correct active & visual pages
                generic_get(i,-1,(uint8*)&i32,4);
            }
            
            if (i32==262){
                for (x=0;x<=255;x++){
                    generic_get(i,-1,(uint8*)&i32,4);
                    sub__palettecolor(x,i32,0,1);
                }
                generic_get(i,-1,(uint8*)&i32,4);
            }
            
            
            //assume command #511("finished screen state") in i32
        }
        
        void chain_savescreenstate(int32 i){//adds the screen state to file #i
            static int32 i32,x,x2;
            static img_struct *i0,*ix;
            i0=&img[page[0]];
            
            if ( (i0->offset>cmem) && (i0->offset<(cmem+1114099)) ){//cmem?[need to maintain cmem state]
                //[256][mode]
                i32=256; generic_put(i,-1,(uint8*)&i32,4);
                i32=i0->compatible_mode; generic_put(i,-1,(uint8*)&i32,4);
                if (i0->text){
                    //[258][WIDTH:X][Y]
                    i32=258; generic_put(i,-1,(uint8*)&i32,4);
                    i32=i0->width; generic_put(i,-1,(uint8*)&i32,4);
                    i32=i0->height; generic_put(i,-1,(uint8*)&i32,4);
                }
                }else{
                //[257][mode][X][Y]
                i32=257; generic_put(i,-1,(uint8*)&i32,4);
                i32=i0->compatible_mode; generic_put(i,-1,(uint8*)&i32,4);
                i32=i0->width; generic_put(i,-1,(uint8*)&i32,4);
                i32=i0->height; generic_put(i,-1,(uint8*)&i32,4);
            }
            //[259][font] (standard fonts only)
            if (i0->font<32){
                i32=259; generic_put(i,-1,(uint8*)&i32,4);
                i32=i0->font; generic_put(i,-1,(uint8*)&i32,4);
            }
            
            //[260][page][rawdata]
            //note: write page is done last to avoid having its values undone by the later page switch
            x2=-1;
            for (x=0;x<pages;x++){
                if (page[x]){
                    if (page[x]!=write_page_index){
                        save_write_page:
                        ix=&img[page[x]];
                        i32=260; generic_put(i,-1,(uint8*)&i32,4);
                        i32=x; generic_put(i,-1,(uint8*)&i32,4);
                        generic_put(i,-1,ix->offset,ix->width*ix->height*ix->bytes_per_pixel);
                        //save structure (specific parts will be reincorporated)
                        generic_put(i,-1,(uint8*)ix,sizeof(img_struct));
                        if (x==x2) break;
                    }else x2=x;
                }
            }
            if ((x2!=-1)&&(x!=x2)){x=x2; goto save_write_page;}
            
            //[261][activepage][visualpage]
            i32=261; generic_put(i,-1,(uint8*)&i32,4);
            i32=0;//note: activepage could be a non-screenpage
            for (x=0;x<pages;x++){
                if (page[x]==write_page_index){i32=x;break;}
            }
            generic_put(i,-1,(uint8*)&i32,4);
            i32=0;
            for (x=0;x<pages;x++){
                if (page[x]==display_page_index){i32=x;break;}
            }
            generic_put(i,-1,(uint8*)&i32,4);
            
            //[262][256x32-bit color palette]
            if (i0->bytes_per_pixel!=4){
                i32=262; generic_put(i,-1,(uint8*)&i32,4);
                for (x=0;x<=255;x++){
                    i32=func__palettecolor(x,0,1); generic_put(i,-1,(uint8*)&i32,4);
                }
            }
            
            //[511](screen state finished)
            i32=511; generic_put(i,-1,(uint8*)&i32,4);
            
        }//chain_savescreenstate
        
        
        
        
        
        
        //GFS file layer: implemented in Rust (crates/qb-runtime/src/gfs.rs)
        int32 gfs_fileno_valid(int32 f){ return qb_gfs_fileno_valid(f); }
        int32 gfs_fileno_freefile(){ return qb_gfs_fileno_freefile(); }
        void gfs_fileno_use(int32 f, int32 i){ qb_gfs_fileno_use(f,i); }
        int32 gfs_open(qbs *filename,int32 access,int32 restrictions, int32 how){ return qb_gfs_open(filename->chr,filename->len,access,restrictions,how); }
        int32 gfs_close(int32 i){ return qb_gfs_close(i); }
        int64 gfs_lof(int32 i){ return qb_gfs_lof(i); }
        int32 gfs_setpos(int32 i, int64 position){ return qb_gfs_setpos(i,position); }
        int64 gfs_getpos(int32 i){ return qb_gfs_getpos(i); }
        int32 gfs_write(int32 i,int64 position,uint8 *data,int64 size){ return qb_gfs_write(i,position,data,size); }
        int32 gfs_read(int32 i,int64 position,uint8 *data,int64 size){ return qb_gfs_read(i,position,data,size); }
        int64 gfs_read_bytes(){ return qb_gfs_read_bytes(); }
        int32 gfs_eof_reached(int32 i){ return qb_gfs_eof_reached(i); }
        int32 gfs_eof_passed(int32 i){ return qb_gfs_eof_passed(i); }

        void sub_lock(int32 i,int64 start,int64 end,int32 passed){
            if (new_error) return;
            int32 e=qb_file_lock(i,start,end,passed,1); if (e) error(e);
        }

        void sub_unlock(int32 i,int64 start,int64 end,int32 passed){
            if (new_error) return;
            int32 e=qb_file_lock(i,start,end,passed,0); if (e) error(e);
        }
        
        // _SCREENIMAGE, _SCREENCLICK and _SCREENPRINT live in the Rust runtime
        // (crates/qb-runtime/src/screen.rs).
        #ifdef DEPENDENCY_SCREENIMAGE
            int32 func__screenimage(int32 x1,int32 y1,int32 x2,int32 y2,int32 passed){
                int32 w,h;
                uint32 *pixels=qb_screenimage(x1,y1,x2,y2,passed,&w,&h);
                if (!pixels&&w>0&&h>0){//no screen capture on this OS: blank image
                    return func__newimage(func_screenwidth(), func_screenheight(), 32, 1);
                }
                static int32 i;
                i=func__newimage(w,h,32,1);
                if (pixels){
                    if (i<-1) memcpy(img[-i].offset32,pixels,(size_t)w*h*4);
                    qb_free_pixels(pixels,(int64)w*h);
                }
                return i;
            }
        #endif //DEPENDENCY_SCREENIMAGE

        void sub__screenclick(int32 x,int32 y, int32 button, int32 passed){
            qb_screenclick(x,y,button,passed);
        }

        void sub__screenprint(qbs *txt){
            qb_screenprint(txt->chr,txt->len);
        }

        // Printer output lives in the Rust runtime (crates/qb-runtime/src/printer.rs,
        // Windows only); scaling the image to the page stays here.
        #ifndef DEPENDENCY_PRINTER
            void sub__printimage(int32 i){
                return;
            }
        #else
            void sub__printimage(int32 i){
                #ifdef QBNex_WINDOWS
                    img_struct *s;
                    int32 x,w,h,i2;
                    if (i>=0){
                        validatepage(i); s=&img[page[i]];
                        }else{
                        x=-i;
                        if (x>=nextimg){error(258); return;}
                        s=&img[x];
                        if (!s->valid){error(258); return;}
                    }
                    void *printer=qb_printer_begin(&w,&h);
                    if (!printer) return;
                    i2=func__newimage(w,h,32,1);
                    if (i2>=-1){qb_printer_abort(printer); return;}//legacy only checked -1; 0 (error) would read img[0]
                    sub__dontblend(i2,1);
                    sub__putimage(NULL,NULL,NULL,NULL,i,i2,NULL,NULL,NULL,NULL,8+32);
                    qb_printer_finish(printer,img[-i2].offset32,w,h);
                    sub__freeimage(i2,1);
                #endif
            }
        #endif
        
        void sub_files(qbs *str,int32 passed){
            //FILES: the listing is built in Rust (crates/qb-runtime/src/fsops.rs), printed here
            if (new_error) return;
            const uint8 *out; int32 len=0;
            int32 e=qb_files(passed?str->chr:NULL,passed?str->len:0,passed,&out,&len);
            if (e==5){error(5); return;}
            std::vector<std::pair<int32,int32>> lines;//(start,length) of each '\n'-separated line
            if (len){
                int32 start=0;
                for (int32 i=0;i<=len;i++){
                    if (i==len||out[i]==10){lines.push_back(std::make_pair(start,i-start)); start=i+1;}
                }
            }
            if (lines.empty()) return;//nothing to list on this OS
            static qbs *strz2=NULL; if (!strz2) strz2=qbs_new(0,0);
            //the current directory
            qbs_set(strz2,qbs_new_txt_len((const char*)out+lines[0].first,lines[0].second));
            qbs_print(strz2,1);
            if (e){error(e); return;}//file not found
            //the entries
            for (size_t k=1;k+1<lines.size();k++){
                qbs_set(strz2,qbs_new_txt_len((const char*)out+lines[k].first,lines[k].second));
                makefit(strz2);
                qbs_print(strz2,0);
            }
            if (func_pos(NULL)>1){strz2->len=0; qbs_print(strz2,1);}//new line if necessary
            qbs_set(strz2,qbs_new_txt_len((const char*)out+lines.back().first,lines.back().second));
            qbs_print(strz2,1);
        }
        
        
        int32 func__keyhit(){
            return qb_keyhit_pop();//R8: keybuf.rs
        }

        int32 func__keydown(int32 x){
            if (x<=0){error(5); return 0;}
            if (keyheld(x)) return -1;
            return 0;
        }
        
        
        
        
        
        
        
        //FIELD: the record buffer and the list of linked strings live in Rust
        //(crates/qb-runtime/src/fileio.rs); only the qbs updates stay here.
        void field_new(int32 fileno){
            int32 e=qb_field_new(fileno,new_error?1:0); if (e) error(e);
        }

        void field_update(int32 fileno){
            //copies the FIELD buffer into every string linked to the file
            static int32 i;
            static gfs_file_struct *gfs;
            i=fileno;
            if (i<0){exit(7701);}//bad file mode (TCP/IP exclusion)
            if (gfs_fileno_valid(i)!=1){exit(7702);}//Bad file name or number
            i=gfs_fileno[i];//convert fileno to gfs index
            gfs=&gfs_file[i];
            if (gfs->type!=1){exit(7703);}//Bad file mode (note: must have RANDOM access)
            static qbs *str;
            for (i=0;i<gfs->field_strings_n;i++){
                str=(qbs*)gfs->field_strings[i];
                if (!str) exit(7704);
                if (str->len!=str->field->size){
                    if (str->len>str->field->size) str->len=str->field->size; else qbs_set(str,qbs_new(str->field->size,1));
                }
                memmove(str->chr,gfs->field_buffer+str->field->offset,str->field->size);
            }
        }

        void lrset_field(qbs *str){
            if (qb_field_store(str->field->fileno,str->field->fileid,str->field->size,str->field->offset,str->chr,str->len)){
                field_update(str->field->fileno);
                return;
            }
            delete str->field;
            str->field=NULL;
        }

        void field_free(qbs* str){
            qb_field_forget(str->field->fileno,str->field->fileid,str);
            delete str->field;
            str->field=NULL;
        }

        void field_add(qbs *str,int64 size){
            qb_field_slot s;
            int32 e=qb_field_add(size,new_error?1:0,&s);
            if (e==-1) return;//an earlier part of this FIELD statement failed
            if (e){error(e); return;}
            if (str->field) field_free(str);
            str->field=new(std::nothrow) qbs_field{};
            if (!str->field){error(518); return;}
            str->field->fileno=s.fileno;
            str->field->fileid=s.fileid;
            str->field->size=s.size;
            str->field->offset=s.offset;
            qb_field_link(s.fileno,str,size);
            field_update(s.fileno);
        }

        void field_get(int32 fileno,int64 offset,int32 passed){
            if (new_error) return;
            int32 e=qb_field_get(fileno,offset,passed);
            if (e){error(e); return;}
            field_update(fileno);
        }

        void field_put(int32 fileno,int64 offset,int32 passed){
            if (new_error) return;
            int32 e=qb_field_put(fileno,offset,passed); if (e) error(e);
        }
        
        
        void sub__mapunicode(int32 unicode_code, int32 ascii_code){
            if (new_error) return;
            if ((unicode_code<0)||(unicode_code>65535)){error(5); return;}
            if ((ascii_code<0)||(ascii_code>255)){error(5); return;}
            codepage437_to_unicode16[ascii_code]=unicode_code;
        }
        
        int32 func__mapunicode(int32 ascii_code){
            if (new_error) return NULL;
            if ((ascii_code<0)||(ascii_code>255)){error(5); return NULL;}
            return (codepage437_to_unicode16[ascii_code]);
        }
        
        
        int32 addone(int32 x){return x+1;}//for testing purposes only
        
        
        qbs *func__os(){
            qbs *tqbs;
            #ifdef QBNex_WINDOWS
                #ifdef QBNex_32
                    tqbs=qbs_new_txt("[WINDOWS][32BIT]");
                    #else
                    tqbs=qbs_new_txt("[WINDOWS][64BIT]");
                #endif
                #elif defined(QBNex_LINUX)
                #ifdef QBNex_32
                    tqbs=qbs_new_txt("[LINUX][32BIT]");
                    #else
                    tqbs=qbs_new_txt("[LINUX][64BIT]");
                #endif
                #elif defined(QBNex_MACOSX)
                #ifdef QBNex_32
                    tqbs=qbs_new_txt("[MACOSX][32BIT][LINUX]");
                    #else
                    tqbs=qbs_new_txt("[MACOSX][64BIT][LINUX]");
                #endif
                #else
                #ifdef QBNex_32
                    tqbs=qbs_new_txt("[32BIT]");
                    #else
                    tqbs=qbs_new_txt("[64BIT]");
                #endif
            #endif
            return tqbs;
        }
        
        int32 func__screenx(){
            #ifdef QBNex_GUI
                while (!window_exists){Sleep(100);} //Wait for window to be created before checking position
                int32 x,y; qb_window_position(&x,&y); return x;
            #endif
            return 0;
        }

        int32 func__screeny(){
            #ifdef QBNex_GUI
                while (!window_exists){Sleep(100);} //Wait for window to be created before checking position
                int32 x,y; qb_window_position(&x,&y); return y;
            #endif
            return 0;
        }

        void sub__screenmove(int32 x,int32 y,int32 passed){
            if (new_error) return;
            if (!passed) goto error;
            if (passed==3) goto error;
            if (full_screen) return;

            #ifdef QBNex_GUI
                while (!window_exists){Sleep(100);} //wait for window to be created before moving it.
                if (passed==2){
                    qb_window_set_position(x,y);
                }else{
                    int32 SW,SH,WW,WH;
                    qb_desktop_size(&SW,&SH);
                    qb_window_size(&WW,&WH);
                    x = (SW - WW)/2;
                    y = (SH - WH)/2;
                    qb_window_set_position(x,y);
                }
            #endif

            return;
            
            error:
            error(5);
        }
        
        void key_update(){
            
            if (key_display_redraw){
                key_display_redraw=0;
                if (!key_display) return;
                }else{
                if (key_display==key_display_state) return;
                key_display_state=key_display;
            }
            
            //use display page 0
            static int32 olddest;
            olddest=func__dest();
            sub__dest(0);
            static img_struct *i;
            i=write_page;
            
            static int32 f,z,c,x2;
            
            //locate bottom-left
            //get current status
            static int32 cx,cy,holding,col1,col2;
            cx=i->cursor_x; cy=i->cursor_y; holding=i->holding_cursor;
            col1=i->color; col2=i->background_color;
            static int32 h,w;
            //calculate height & width in characters
            if (i->compatible_mode){
                h=i->height/fontheight[i->font];
                if (fontwidth[i->font]){
                    w=i->width/fontwidth[i->font];
                    }else{
                    w=write_page->width;
                }
                }else{
                h=i->height;
                w=i->width;
            }
            i->cursor_x=1; i->cursor_y=h; i->holding_cursor=0;
            
            static qbs *str=NULL; if (!str) str=qbs_new(0,0);
            static qbs *str2=NULL; if (!str2) str2=qbs_new(1,0);
            
            //clear bottom row using background color
            if (i->text){
                for (x2=1;x2<=i->width;x2++){
                    str2->chr[0]=32; qbs_print(str2,0);
                }
                i->cursor_x=1; i->cursor_y=h; i->holding_cursor=0;
                }else{
                fast_boxfill(0,(i->cursor_y-1)*fontheight[i->font],i->width-1,i->cursor_y*fontheight[i->font]-1,col2|0xFF000000);
            }
            
            if (!key_display) goto no_key;
            
            static int32 item_x,limit_x,row_limit,leeway_x;
            leeway_x=0;
            if (i->compatible_mode){
                if (fontwidth[i->font]){
                    item_x=w/12; row_limit=item_x*12;
                    if (item_x<8){//cannot fit min. width
                        item_x=8; row_limit=(w/8)*8;
                        if (item_x>w){item_x=w; row_limit=w;}//can't even fit 1!
                    }
                    }else{
                    leeway_x=fontheight[i->font];
                    item_x=w/12; row_limit=item_x*12-leeway_x;
                    x2=((float)fontheight[i->font])*0.5;//estimate the average character width (it's OK for this to be wrong)
                    if (item_x<(x2*8+leeway_x)){//cannot fit min. width
                        item_x=(x2*8+leeway_x); row_limit=(w/(x2*8+leeway_x))*(x2*8+leeway_x)-leeway_x;
                        if (item_x>w){item_x=w; row_limit=w-leeway_x;}//can't even fit 1!
                    }  
                }
                }else{
                item_x=w/12; row_limit=item_x*12;
                if (item_x<8){//cannot fit min. width
                    item_x=8; row_limit=(w/8)*8;
                    if (item_x>w){item_x=w; row_limit=w;}//can't even fit 1!
                }
            }
            
            static int32 final_chr,row_final_chr;
            
            row_final_chr=0;
            for (f=1;f<=12;f++){
                final_chr=0;
                limit_x=f*item_x-leeway_x;//set new limit
                
                //relocate
                x2=((f-1)*item_x)+1;
                if (x2>=row_limit){row_final_chr=1; goto done_f;}
                i->cursor_x=x2;
                
                //number string
                if (fontwidth[i->font]){
                    qbs_set(str,qbs_ltrim(qbs_str(f)));
                    }else{
                    qbs_set(str,qbs_add(qbs_ltrim(qbs_str(f)),qbs_new_txt(")")));
                }
                for (z=0;z<str->len;z++){
                    if (i->cursor_x>=row_limit) row_final_chr=1;
                    if (i->cursor_x>limit_x) goto done_f;
                    if (i->cursor_x>=limit_x) final_chr=1;
                    str2->chr[0]=str->chr[z]; qbs_print(str2,0);
                    if (final_chr) goto done_f;
                }
                
                //text
                static int32 fi;
                fi=f; if (f>10) fi=f-11+30;
                if (onkey[fi].text){
                    qbs_set(str,onkey[fi].text);
                    if (i->text){
                        if (i->background_color){
                            i->color=7; i->background_color=0;
                            }else{
                            i->color=0; i->background_color=7;
                        }
                    }
                    }else{
                    str->len=0;
                }
                z=0;
                while(i->cursor_x<limit_x){
                    static int32 c;
                    
                    if (z>=str->len){
                        if (!onkey[fi].text) goto done_f;
                        c=32;
                        }else{
                        c=str->chr[z++];
                    }
                    
                    if (i->cursor_x>=row_limit) row_final_chr=1;
                    if (i->cursor_x>limit_x) goto done_f;
                    if (i->cursor_x>=limit_x) final_chr=1;
                    /*
                        7->14
                        8->254
                        9->26
                        10->60
                        11->127
                        12->22
                        13->27
                        28->16
                        29->17
                        30->24
                        31->25
                        KEY LIST puts spaces instead of non-printables
                        QBASIC's KEY LIST differs from QBX in this regard
                        CHR$(13) is also turned into a space in KEY LIST, even if it is at the end
                    */
                    if (c==7) c=14;
                    if (c==8) c=254;
                    if (c==9) c=26;
                    if (c==10) c=60;
                    if (c==11) c=127;
                    if (c==12) c=22;
                    if (c==13) c=27;
                    if (c==28) c=16;
                    if (c==29) c=17;
                    if (c==30) c=24;
                    if (c==31) c=25;
                    str2->chr[0]=c;
                    no_control_characters=1; qbs_print(str2,0); no_control_characters=0;
                    if (final_chr) goto done_f;
                }
                
                done_f:;
                i->color=col1; i->background_color=col2;
                if (row_final_chr) goto done_row;
            }
            done_row:;
            
            //revert status
            no_key:
            i->cursor_x=cx; i->cursor_y=cy; i->holding_cursor=holding; i->color=col1; i->background_color=col2;
            sub__dest(olddest);
        }
        
        void key_on(){
            key_display=1; key_update();
        }
        
        void key_off(){
            key_display=0; key_update();
        }
        
        void key_list(){
            static img_struct *i;
            i=write_page;
            static int32 mono;
            mono=1; if (!fontwidth[i->font]) if (func__printwidth(qbs_new_txt(" "),NULL,NULL)!=func__printwidth(qbs_new_txt(")"),NULL,NULL)) mono=0;
            static int32 f,fi;
            static qbs *str=NULL; if (!str) str=qbs_new(0,0);
            for (f=1;f<=12;f++){
                
                //F-number & spacer
                if (fontwidth[i->font]){
                    if (f<10){
                        qbs_set(str,qbs_add(qbs_ltrim(qbs_str(f)),qbs_new_txt("  ")));
                        }else{
                        qbs_set(str,qbs_add(qbs_ltrim(qbs_str(f)),qbs_new_txt(" ")));
                    }
                    }else{
                    if ((f<10)&&(mono==1)){
                        qbs_set(str,qbs_add(qbs_ltrim(qbs_str(f)),qbs_new_txt(")  ")));
                        }else{
                        qbs_set(str,qbs_add(qbs_ltrim(qbs_str(f)),qbs_new_txt(") ")));
                    }
                }
                qbs_set(str,qbs_add(qbs_new_txt("F"),str));
                
                
                //text
                fi=f; if (f>10) fi=f-11+30;
                if (onkey[fi].text){
                    qbs_print(str,0);
                    /*
                        7->14
                        8->254
                        9->26
                        10->60
                        11->127
                        12->22
                        13->27
                        28->16
                        29->17
                        30->24
                        31->25
                        KEY LIST puts spaces instead of non-printables
                        QBASIC's KEY LIST differs from QBX in this regard
                        CHR$(13) is also turned into a space in KEY LIST, even if it is at the end
                    */
                    str->len=1;
                    static int32 x,c;
                    for (x=0;x<onkey[fi].text->len;x++){
                        c=onkey[fi].text->chr[x];
                        if ((c>=7)&&(c<=13)) c=32;
                        if ((c>=28)&&(c<=31)) c=32;
                        str->chr[0]=c;
                        qbs_print(str,0);
                    }
                    str->len=0; qbs_print(str,1);
                    }else{
                    qbs_print(str,1);
                }
                
            }//f
        }
        
        void key_assign(int32 i,qbs *str){
            if (new_error) return;
            static int32 x,x2,i2;
            
            if ( ((i>=1)&&(i<=10)) || (i==30) || (i==31) ){//F1-F10,F11,F12
                if (str->len>15){error(5); return;}
                if (!onkey[i].text) onkey[i].text=qbs_new(0,0);
                qbs_set(onkey[i].text,str);
                key_display_redraw=1; key_update();
                return;
            }//F1-F10,F11,F12
            
            if ((i>=15)&&(i<=29)){//user defined key
                if (str->len==0){
                    onkey[i].key_scancode=0;
                    }else{
                    x=str->chr[str->len-1];
                    x2=0; for (i2=0;i2<str->len-1;i2++) x2|=str->chr[i2];
                    onkey[i].key_scancode=x;
                    onkey[i].key_flags=x2;
                }
                return;
            }//user defined key
            
            error(5);
            return;
        }
        
        void sub_paletteusing(void *element,int32 bits){
            //note: bits is either 16(INTEGER) or 32(LONG)
            if (new_error) return;
            static byte_element_struct *ele; ele=(byte_element_struct*)element;
            static int16 *i16; i16=(int16*)ele->offset;
            static int32 *i32; i32=(int32*)ele->offset;
            if (write_page->bits_per_pixel==32) goto error;
            static int32 last_color,i,c;
            last_color=write_page->mask;
            if (ele->length<((bits/8)*(last_color+1))) goto error;
            if ((write_page->compatible_mode==11)||(write_page->compatible_mode==12)||(write_page->compatible_mode==13)||(write_page->compatible_mode==256)){
                if (bits==16) goto error;//must be an array of type LONG in these modes
            }
            for (i=0;i<=last_color;i++){
                if (bits==16){c=*i16; i16++;}else{c=*i32; i32++;}
                if (c<-1) goto error;
                if (c!=-1){
                    qbg_palette(i,c,1);
                    if (new_error) return;
                }
            }
            return;
            error:
            
            error(5);
        }
        
        
        void sub__depthbuffer(int32 options,int32 dst,int32 passed){
            //                    {ON|OFF|LOCK|_CLEAR}
            
            if (new_error) return;
            
            if ((passed&1)==0) dst=0;//the primary hardware surface is implied
            hardware_img_struct* dst_himg=NULL;
            if (dst<0){
                dst_himg=(hardware_img_struct*)list_get(hardware_img_handles,dst-HARDWARE_IMG_HANDLE_OFFSET);
                if (dst_himg==NULL){error(258); return;}
                dst-=HARDWARE_IMG_HANDLE_OFFSET;
                }else{
                if (dst>1) {error(5); return;}
                dst=-dst;
            }
            
            if (options==4){
                flush_old_hardware_commands();
                int32 hgch=list_add(hardware_graphics_command_handles);
                hardware_graphics_command_struct* hgc=(hardware_graphics_command_struct*)list_get(hardware_graphics_command_handles,hgch);
                hgc->remove=0;
                //set command values
                hgc->command=HARDWARE_GRAPHICS_COMMAND__CLEAR_DEPTHBUFFER;
                hgc->dst_img=dst;
                //queue the command
                hgc->next_command=0;
                hgc->order=display_frame_order_next;
                if (last_hardware_command_added){
                    hardware_graphics_command_struct* hgc2=(hardware_graphics_command_struct*)list_get(hardware_graphics_command_handles,last_hardware_command_added);
                    hgc2->next_command=hgch;
                }
                last_hardware_command_added=hgch;
                if (first_hardware_command==0) first_hardware_command=hgch;
                return;
            }
            
            int32 new_mode;
            if (options==1){
                new_mode=DEPTHBUFFER_MODE__ON;
            }
            if (options==2){
                new_mode=DEPTHBUFFER_MODE__OFF;
            }
            if (options==3){
                new_mode=DEPTHBUFFER_MODE__LOCKED;
            }
            
            if (dst==0){
                depthbuffer_mode0=new_mode;
                return;
            }
            if (dst==-1){
                depthbuffer_mode1=new_mode;
                return;
            }
            dst_himg->depthbuffer_mode=new_mode;
        }
        
        void sub__maptriangle(int32 cull_options,float sx1,float sy1,float sx2,float sy2,float sx3,float sy3,int32 si,float fdx1,float fdy1,float fdz1,float fdx2,float fdy2,float fdz2,float fdx3,float fdy3,float fdz3,int32 di,int32 smooth_options,int32 passed){
            //[{_CLOCKWISE|_ANTICLOCKWISE}][{_SEAMLESS}](?,?)-(?,?)-(?,?)[,?]{TO}(?,?[,?])-(?,?[,?])-(?,?[,?])[,[?][,{_SMOOTH|_SMOOTHSHRUNK|_SMOOTHSTRETCHED}]]"
            //  (1)       (2)              1                             2           4         8         16    32   (1)     (2)           (3)
            
            if (new_error) return;
            
            static int32 dwidth,dheight,swidth,sheight,swidth2,sheight2;
            static int32 lhs,rhs,lhs1,lhs2,top,bottom,temp,flats,flatg,final,tile,no_edge_overlap;
            flats=0; final=0; tile=0; no_edge_overlap=0;
            static int32 v,i,x,x1,x2,y,y1,y2,z,h,ti,lhsi,rhsi,d; 
            static int32 g1x,g2x,g1tx,g2tx,g1ty,g2ty,g1xi,g2xi,g1txi,g2txi,g1tyi,g2tyi,tx,ty,txi,tyi,roff,loff;
            static int64 i64;
            static img_struct *src,*dst;
            static uint8* pixel_offset;
            static uint32* pixel_offset32;
            static uint8* dst_offset;
            static uint32* dst_offset32;
            static uint8* src_offset;
            static uint32* src_offset32;
            static uint32 col,destcol,transparent_color;
            static uint8* cp;
            
            //hardware support
            //is source a hardware handle?
            if (si){
                
                static int32 src,dst;//scope is a wonderful thing
                src=si;
                dst=di;
                hardware_img_struct* src_himg=(hardware_img_struct*)list_get(hardware_img_handles,src-HARDWARE_IMG_HANDLE_OFFSET);
                if (src_himg!=NULL){//source is hardware image
                    src-=HARDWARE_IMG_HANDLE_OFFSET;
                    
                    flush_old_hardware_commands();
                    
                    //check dst
                    hardware_img_struct* dst_himg=NULL;
                    if (dst<0){
                        dst_himg=(hardware_img_struct*)list_get(hardware_img_handles,dst-HARDWARE_IMG_HANDLE_OFFSET);
                        if (dst_himg==NULL){error(258); return;}
                        dst-=HARDWARE_IMG_HANDLE_OFFSET;
                        }else{
                        if (dst>1) {error(5); return;}
                        dst=-dst;
                    }
                    
                    static int32 use3d;
                    use3d=0;
                    if (passed&(4+8+16)) use3d=1;
                    
                    if ((passed&1)==1&&use3d==0){error(5);return;}//seamless not supported for 2D hardware version yet
                    
                    //create new command handle & structure
                    int32 hgch=list_add(hardware_graphics_command_handles);
                    hardware_graphics_command_struct* hgc=(hardware_graphics_command_struct*)list_get(hardware_graphics_command_handles,hgch);
                    
                    hgc->remove=0;
                    
                    //set command values
                    if (use3d){
                        hgc->command=HARDWARE_GRAPHICS_COMMAND__MAPTRIANGLE3D;
                        hgc->cull_mode=CULL_MODE__NONE;
                        if (cull_options==1) hgc->cull_mode=CULL_MODE__CLOCKWISE_ONLY;
                        if (cull_options==2) hgc->cull_mode=CULL_MODE__ANTICLOCKWISE_ONLY;
                        }else{
                        hgc->command=HARDWARE_GRAPHICS_COMMAND__MAPTRIANGLE;
                    }
                    
                    hgc->src_img=src;
                    hgc->src_x1=sx1;
                    hgc->src_y1=sy1;
                    hgc->src_x2=sx2;
                    hgc->src_y2=sy2;
                    hgc->src_x3=sx3;
                    hgc->src_y3=sy3;
                    
                    hgc->dst_img=dst;
                    hgc->dst_x1=fdx1;
                    hgc->dst_y1=fdy1;
                    hgc->dst_x2=fdx2;
                    hgc->dst_y2=fdy2;
                    hgc->dst_x3=fdx3;
                    hgc->dst_y3=fdy3;
                    if (use3d){
                        hgc->dst_z1=fdz1;
                        hgc->dst_z2=fdz2;
                        hgc->dst_z3=fdz3;
                        if (dst==0) hgc->depthbuffer_mode=depthbuffer_mode0;
                        if (dst==-1) hgc->depthbuffer_mode=depthbuffer_mode1;
                        if (dst_himg!=NULL){
                            hgc->depthbuffer_mode=dst_himg->depthbuffer_mode;
                        }        
                    }
                    
                    hgc->smooth=smooth_options;
                    
                    hgc->use_alpha=1;
                    if (src_himg->alpha_disabled) hgc->use_alpha=0;
                    //only consider dest alpha setting if it is a hardware image
                    if (dst_himg!=NULL){
                        if (dst_himg->alpha_disabled) hgc->use_alpha=0;
                    }
                    
                    //queue the command
                    hgc->next_command=0;
                    hgc->order=display_frame_order_next;
                    
                    if (last_hardware_command_added){
                        hardware_graphics_command_struct* hgc2=(hardware_graphics_command_struct*)list_get(hardware_graphics_command_handles,last_hardware_command_added);
                        hgc2->next_command=hgch;
                    }
                    last_hardware_command_added=hgch;
                    if (first_hardware_command==0) first_hardware_command=hgch;
                    
                    return;
                    
                }
            }
            
            if (passed&(4+8+16)){error(5);return;}//3D not supported using software surfaces
            
            //recreate old calling convention
            static int32 passed_original;
            passed_original=passed;
            passed=0;
            if (passed_original&1) passed+=1;
            if (passed_original&2) passed+=2;
            if (passed_original&32) passed+=4;
            if (passed_original&64) passed+=8;
            
            
            //get/validate src/dst images
            if (passed&2){
                if (si>=0){//validate si
                    validatepage(si); si=page[si];
                    }else{
                    si=-si; if (si>=nextimg){error(258); return;} if (!img[si].valid){error(258); return;}
                }
                src=&img[si];
                }else{
                src=read_page;
            }
            if (passed&4){
                if (di>=0){//validate di
                    validatepage(di); di=page[di];
                    }else{
                    di=-di; if (di>=nextimg){error(258); return;} if (!img[di].valid){error(258); return;}
                }
                dst=&img[di];
                }else{
                dst=write_page;
            }
            //clipping, texture mapping and the pixel loops: Rust (qb-runtime gfx/maptri.rs)
            float s6[6]={sx1,sy1,sx2,sy2,sx3,sy3};
            float d6[6]={fdx1,fdy1,fdx2,fdy2,fdx3,fdy3};
            if (int32 e=qb_gfx_maptriangle(src,dst,s6,d6,passed&1)) error(e);
        }//sub__maptriangle
        
        extern int32 func__devices();
        
        int32 func_stick(int32 i,int32 axis_group,int32 passed){
            //note: range: 1-254 (127=neutral), top-left to bottom-right positive
            //             128 returned for unattached devices
            //QBNex extension: 'i' allows for joystick selection 0,1->JoyA, 2,3->JoyB, 4,5->JoyC, etc
            //                'axis_group' selects the pair of axes to read from, 1 is the default
            if (device_last==0) func__devices();//init device interface (if not already setup)
            if (passed){
                if (axis_group<1||axis_group>65535){error(5); return 0;}
                }else{
                axis_group=1;
            }
            if (i<0||i>65535){error(5); return 0;}
            static int32 di,axis,i2,v;
            static device_struct *d;
            static float f;
            axis=(i&1)+(axis_group-1)*2;
            i=i>>1;
            i2=0;
            for(di=1;di<=device_last;di++){
                d=&devices[di];
                if (d->type==1){
                    if (i==i2){
                        if (axis<d->lastaxis){
                            f=getDeviceEventAxisValue(d,d->queued_events-1,axis);
                            if (f>-0.01&&f<=0.01) f=0;
                            v=qbr_float_to_long(f*127.0)+127;
                            if (v>254) v=254;
                            if (v<1) v=1;
                            return v;
                        }//axis valid
                    }
                    i2++;
                }//type==1
            }//di
            return 128;
        }
        
        int32 func_strig(int32 i,int32 controller,int32 passed){
            //note: returns 0 or -1(true)
            //QBNex extension: 'i' refers to a button (b1,b1,b1,b1,b2,b2,b2,b2,b3,b3,b3,b3,b4,...)
            //                'controller' overrides the controller implied by 'i', 1 is the default
            if (device_last==0) func__devices();//init device interface (if not already setup)
            if (i<0||i>65535){error(5); return 0;}
            if (passed){
                if (controller<1||controller>65535){error(5); return 0;}
                }else{
                controller=1; if (i&2){controller=2; i-=2;}
            }
            static int32 di,button,method,c,v;
            static device_struct *d;
            button=(i>>2)+1;
            method=(i&1)+1;//1=if pressed since last call, 2=currently down
            c=1;
            for(di=1;di<=device_last;di++){
                d=&devices[di];
                if (d->type==1){
                    if (c==controller){
                        if (button<=d->lastbutton){//button exists
                            if (method==1){
                                //method 1: pressed since last call
                                if (button>0&&button<=256){
                                    if (d->STRIG_button_pressed[button-1]){
                                        d->STRIG_button_pressed[button-1]=0;
                                        return -1;
                                    }
                                }
                                return 0;
                                }else{
                                //method 2: currently down
                                v=getDeviceEventButtonValue(d,d->queued_events-1,button-1);
                                if (v) return -1; else return 0;
                            }
                        }//button exists
                    }//c==controller
                    c++;
                }//type==1
            }//di
            return 0;
        }
        
        int32 func__fileexists(qbs* file){//Rust: crates/qb-runtime/src/fsops.rs
            if (new_error) return 0;
            return qb_exists(file->chr,file->len,0);
        }

        int32 func__direxists(qbs* file){//Rust: crates/qb-runtime/src/fsops.rs
            if (new_error) return 0;
            return qb_exists(file->chr,file->len,1);
        }
        
        int32 func__console(){
            if (new_error) return -1;
            return console_image;
        }
        
        void sub__console(int32 onoff){//on=1 off=2
            if (!console) return;//command does nothing if console unavailable
            if (onoff==1){
                //turn on
                if (!console_active){
                    #ifdef QBNex_WINDOWS
                        if (console_child){
                            ShowWindow( GetConsoleWindow(), SW_SHOWNOACTIVATE );
                        }
                    #endif
                    console_active=1;
                }
                }else{
                //turn off
                if (console_active){
                    #ifdef QBNex_WINDOWS
                        if (console_child){
                            ShowWindow( GetConsoleWindow(), SW_HIDE );
                        }
                    #endif
                    console_active=0;
                }
            }
            
        }
        
        
        void sub__screenshow(){
            #ifdef QBNex_GUI
                qb_window_show(1);
            #endif
            create_window=1;
            screen_hide=0;
        }

        void sub__screenhide(){
            #ifdef QBNex_GUI
                qb_window_show(0);
            #endif
            screen_hide=1;
        }

        int32 func__screenhide(){return -screen_hide;}
        
        void sub__consoletitle(qbs* s){
            #ifdef QBNex_WINDOWS
                std::string title((char*)s->chr, (char*)s->chr + s->len);
                if (console){ if (console_active){
                    SetConsoleTitle(title.c_str());
                    Sleep(40);
                }}
            #endif
        }
        
        
        
        //_MEMFREE, _MEM(offset,size), _MEMNEW: implemented in Rust (crates/qb-runtime/src/mem.rs)
        void sub__memfree(void *mem){ int32 e=qb_memfree((qb_mem_block*)mem); if (e) error(e); }
        extern mem_block func__mem_at_offset(ptrszint offset,ptrszint size){
            mem_block b; int32 e=qb_mem_at_offset(offset,size,new_error?1:0,(qb_mem_block*)&b); if (e) error(e); return b;
        }
        mem_block func__memnew(ptrszint bytes){
            mem_block b; int32 e=qb_memnew(bytes,new_error?1:0,(qb_mem_block*)&b); if (e) error(e); return b;
        }
        
        
        mem_block func__memimage(int32 i,int32 passed){
            
            static mem_block b;
            
            if (new_error) goto error;
            
            static int image_handle;
            
            static img_struct *im;
            if (passed){
                if (i>=0){
                    validatepage(i); im=&img[image_handle=page[i]]; 
                    image_handle=-image_handle;
                    }else{
                    image_handle=i;
                    i=-i;
                    if (i>=nextimg){error(258); goto error;}
                    im=&img[i];
                    if (!im->valid){error(258); goto error;}
                }
                }else{
                im=write_page;
            }
            
            if (im->lock_id){
                b.lock_offset=(ptrszint)im->lock_offset; b.lock_id=im->lock_id;//get existing tag
                }else{
                new_mem_lock();
                mem_lock_tmp->type=2;//image
                b.lock_offset=(ptrszint)mem_lock_tmp; b.lock_id=mem_lock_id;
                im->lock_offset=(void*)mem_lock_tmp; im->lock_id=mem_lock_id;//create tag
            }
            
            b.offset=(ptrszint)im->offset;
            b.size=im->bytes_per_pixel*im->width*im->height;
            b.type=im->bytes_per_pixel+128+1024+2048;//integer+unsigned+pixeltype
            b.elementsize=im->bytes_per_pixel;
            b.image=image_handle;
            
            return b;
            error:
            b.offset=0;
            b.size=0;
            b.lock_offset=(ptrszint)&mem_lock_invalid; b.lock_id=MEM_LOCK_INVALID_ID;//set invalid lock
            b.type=0;
            b.elementsize=0;
            b.image=-1;
            return b;
        }
        
        //_MEM functions: implemented in Rust (crates/qb-runtime/src/mem.rs)
        int32 func__memexists(void* void_blk){ return qb_memexists((qb_mem_block*)void_blk); }
        void *func__memget(mem_block* blk,ptrszint off,ptrszint bytes){
            int32 e=0; void *p=qb_memget((qb_mem_block*)blk,off,bytes,&e); if (e) error(e); return p;
        }
        void sub__memfill_nochecks(ptrszint doff,ptrszint dbytes,ptrszint soff,ptrszint sbytes){ qb_memfill_nochecks(doff,dbytes,(const uint8*)soff,sbytes); }
        void sub__memfill(mem_block* dblk,ptrszint doff,ptrszint dbytes,ptrszint soff,ptrszint sbytes){
            int32 e=qb_memfill((qb_mem_block*)dblk,doff,dbytes,(const uint8*)soff,sbytes); if (e) error(e);
        }
        void sub__memfill_1(mem_block* dblk,ptrszint doff,ptrszint dbytes,int8 val){ sub__memfill(dblk,doff,dbytes,(ptrszint)&val,1); }
        void sub__memfill_nochecks_1(ptrszint doff,ptrszint dbytes,int8 val){ sub__memfill_nochecks(doff,dbytes,(ptrszint)&val,1); }
        void sub__memfill_2(mem_block* dblk,ptrszint doff,ptrszint dbytes,int16 val){ sub__memfill(dblk,doff,dbytes,(ptrszint)&val,2); }
        void sub__memfill_nochecks_2(ptrszint doff,ptrszint dbytes,int16 val){ sub__memfill_nochecks(doff,dbytes,(ptrszint)&val,2); }
        void sub__memfill_4(mem_block* dblk,ptrszint doff,ptrszint dbytes,int32 val){ sub__memfill(dblk,doff,dbytes,(ptrszint)&val,4); }
        void sub__memfill_nochecks_4(ptrszint doff,ptrszint dbytes,int32 val){ sub__memfill_nochecks(doff,dbytes,(ptrszint)&val,4); }
        void sub__memfill_8(mem_block* dblk,ptrszint doff,ptrszint dbytes,int64 val){ sub__memfill(dblk,doff,dbytes,(ptrszint)&val,8); }
        void sub__memfill_nochecks_8(ptrszint doff,ptrszint dbytes,int64 val){ sub__memfill_nochecks(doff,dbytes,(ptrszint)&val,8); }
        void sub__memfill_SINGLE(mem_block* dblk,ptrszint doff,ptrszint dbytes,float val){ sub__memfill(dblk,doff,dbytes,(ptrszint)&val,4); }
        void sub__memfill_nochecks_SINGLE(ptrszint doff,ptrszint dbytes,float val){ sub__memfill_nochecks(doff,dbytes,(ptrszint)&val,4); }
        void sub__memfill_DOUBLE(mem_block* dblk,ptrszint doff,ptrszint dbytes,double val){ sub__memfill(dblk,doff,dbytes,(ptrszint)&val,8); }
        void sub__memfill_nochecks_DOUBLE(ptrszint doff,ptrszint dbytes,double val){ sub__memfill_nochecks(doff,dbytes,(ptrszint)&val,8); }
        static uint8 memfill_FLOAT_padding[32];//_FLOAT pattern: the value followed by null bytes (32 bytes)
        void sub__memfill_FLOAT(mem_block* dblk,ptrszint doff,ptrszint dbytes,long double val){ *(long double*)memfill_FLOAT_padding=val; sub__memfill(dblk,doff,dbytes,(ptrszint)memfill_FLOAT_padding,32); }
        void sub__memfill_nochecks_FLOAT(ptrszint doff,ptrszint dbytes,long double val){ *(long double*)memfill_FLOAT_padding=val; sub__memfill_nochecks(doff,dbytes,(ptrszint)memfill_FLOAT_padding,32); }
        void sub__memfill_OFFSET(mem_block* dblk,ptrszint doff,ptrszint dbytes,ptrszint val){ sub__memfill(dblk,doff,dbytes,(ptrszint)&val,sizeof(ptrszint)); }
        void sub__memfill_nochecks_OFFSET(ptrszint doff,ptrszint dbytes,ptrszint val){ sub__memfill_nochecks(doff,dbytes,(ptrszint)&val,sizeof(ptrszint)); }
        void sub__memcopy(void *sblk,ptrszint soff,ptrszint bytes,void *dblk,ptrszint doff){
            int32 e=qb_memcopy((qb_mem_block*)sblk,soff,bytes,(qb_mem_block*)dblk,doff); if (e) error(e);
        }
        mem_block func__mem(ptrszint offset,ptrszint size,int32 type,ptrszint elementsize,mem_lock *lock){
            mem_block b; qb_mem(offset,size,type,elementsize,(qb_mem_lock*)lock,(qb_mem_block*)&b); return b;
        }
        
        /* Extra maths functions - we do what we must because we can */
        double func_deg2rad (double value) {
            return (value * 0.01745329251994329576923690768489);
        }
        
        double func_rad2deg (double value) {
            return (value * 57.29577951308232);
        } 
        
        double func_deg2grad (double value) {
            return (value * 1.111111111111111);
        }
        
        double func_grad2deg (double value) {
            return (value * 0.9);
        }
        
        double func_rad2grad (double value) {
            return (value * 63.66197723675816);
        } 
        
        double func_grad2rad (double value) {
            return (value * .01570796326794896);
        } 
        
        double func_pi (double multiplier,int32 passed) {
            if (passed) {return 3.14159265358979323846264338327950288419716939937510582 * multiplier;}
            return (3.14159265358979323846264338327950288419716939937510582);
        }
        
        double func_arcsec (double num) {
            int sign = (num > 0) - (num < 0);
            if (num<-1||num>1) {error(5);return 0;}
            return atan(num / sqrt(1 - num * num)) + (sign - 1) * (2 * atan(1));
        }
        
        double func_arccsc (double num) {
            int sign = (num > 0) - (num < 0);
            if (num<-1||num>1) {error(5);return 0;}
            return atan(num / sqrt(1 - num * num)) + (sign - 1) * (2 * atan(1));
        }
        
        double func_arccot (double num) {return 2 * atan(1) - atan(num);}
        
        double func_sech (double num) {
            if (num>88.02969) {error(5);return 0;}
            if (exp(num) + exp(-num)==0) {error(5);return 0;}
            return 2/ (exp(num) + exp(-num));
        }
        
        double func_csch (double num) {
            if (num>88.02969) {error(5);return 0;}
            if (exp(num) - exp(-num)==0) {error(5);return 0;}
            return 2/ (exp(num) - exp(-num));
        }
        
        double func_coth (double num) {
            if (num>44.014845) {error(5);return 0;}
            if (2 * exp(num) - 1==0) {error(5);return 0;}
            return 2 * exp(num) - 1;
        }
        
        double func_sec (double num) {
            if (cos(num)==0) {error(5);return 0;}
            return 1/cos(num);
        }
        
        double func_csc (double num) {
            if (sin(num)==0) {error(5);return 0;}
            return 1/sin(num);
        }
        
        double func_cot (double num) {
            if (tan(num)==0) {error(5);return 0;}
            return 1/tan(num);
        }
        
        
//R8: GLUT keyboard/timer callbacks replaced by qb-runtime keymap.rs + window_bridge.cpp
#include "libqb/window_bridge.cpp"

void sub__title(qbs *title){
    if (new_error) return;
    static qbs *cz=NULL;
    if (!cz){cz=qbs_new(1,0); cz->chr[0]=0;}
    static qbs *str=NULL; if (!str) str=qbs_new(0,0);
    qbs_set(str,qbs_add(title,cz));
    
    window_title_storage.assign(str->chr,str->chr+str->len);
    window_title=window_title_storage.empty()?NULL:window_title_storage.data();
    
    #ifdef QBNex_GUI
        qb_window_set_title(window_title?(char*)window_title:"");
    #endif
    
}//title


void sub__echo(qbs *message){
    if (new_error) return;
    
    int32 prevDest=func__dest();
    sub__dest(func__console());
    
    makefit(message);
    qbs_print(message,0);
    qbs_print(nothingstring,1);
    
    sub__dest(prevDest);
    
}//echo

void sub__filedrop(int32 on_off=NULL) {
    #ifdef QBNex_WINDOWS
        if ((on_off==NULL)||(on_off==1)) acceptFileDrop=-1;
        if ((on_off==2)) acceptFileDrop=0;
    #endif
}

int32 func__filedrop() {
    return acceptFileDrop;
}

void sub__finishdrop() {
    qb_input_lock();
    dropped_files.clear();
    qb_input_unlock();
}

int32 func__totaldroppedfiles() {
    qb_input_lock();
    int32 n=(int32)dropped_files.size();
    qb_input_unlock();
    return n;
}

qbs *func__droppedfile(int32 fileIndex, int32 passed) {
    static int32 index=-1;
    std::string name;
    int32 total=func__totaldroppedfiles();
    if (total > 0) {
        index++;
        if (passed) index=fileIndex-1;
        if ((index>total-1)||(index<0)) {
            //out of bounds; reading sequentially (without an index) clears the list
            if (!passed) sub__finishdrop();
            index=-1;
            return qbs_new_txt("");
        }
        qb_input_lock();
        name=dropped_files[index];
        qb_input_unlock();
        if ((!passed)&&(index==total-1)) {
            //last file read sequentially
            sub__finishdrop();
            index=-1;
        }
        return qbs_new_txt(name.c_str());
    }
    index=-1;
    return qbs_new_txt("");
}

//                     0 1  2        0 1       2
void sub__resize(int32 on_off, int32 stretch_smooth){
    
    if (on_off==1) resize_snapback=0;
    if (on_off==2) resize_snapback=1;
    //no change if omitted
    
    if (stretch_smooth){
        resize_auto=stretch_smooth;
        }else{
        resize_auto=0;//revert if omitted
    }
    
}

int32 func__resize(){
    if (resize_snapback) return 0; //resize must be enabled
    if (resize_event){
        resize_event=0;
        return -1;
    } 
    return 0;
}

int32 func__resizewidth(){
    return resize_event_x;
}
int32 func__resizeheight(){
    return resize_event_y;
}

int32 func__scaledwidth(){
    return environment_2d__screen_scaled_width;
}
int32 func__scaledheight(){
    return environment_2d__screen_scaled_height;
}



//Get Current Working Directory
qbs *func__cwd(){//Get Current Working Directory (Rust: crates/qb-runtime/src/fsops.rs)
    const uint8 *out; int32 len=0;
    int32 e=qb_cwd(&out,&len);
    if (e==7){error(7); return NULL;}//"Out of memory" (legacy returned no string)
    if (e){qbs *tqbs=qbs_new(0,1); error(e); return tqbs;}//51 "Internal error"
    qbs *final=qbs_new(len,1);
    memcpy(final->chr,out,len);
    return final;
}

qbs *startDir=NULL;//set on startup
qbs *func__startdir(){
    qbs *temp=qbs_new(0, 1);
    qbs_set(temp, startDir);
    return temp;
}

qbs *rootDir=NULL;//the dir moved to when program begins

qbs *func__dir(qbs* context_in){
    
    static qbs *context=NULL;
	if (!context){context=qbs_new(0,0);}
    
	qbs_set(context,qbs_ucase(context_in));
    
	if (qbs_equal(qbs_ucase(context),qbs_new_txt("TEXT"))||qbs_equal(qbs_ucase(context),qbs_new_txt("DOCUMENT"))||qbs_equal(qbs_ucase(context),qbs_new_txt("DOCUMENTS"))||qbs_equal(qbs_ucase(context),qbs_new_txt("MY DOCUMENTS"))){
		#ifdef QBNex_WINDOWS
			CHAR osPath[MAX_PATH];
			if(SUCCEEDED(SHGetFolderPathA(NULL,5,NULL,0,osPath))){ //Documents
				return qbs_add(qbs_new_txt(osPath),qbs_new_txt("\\"));
            }
        #endif
    }
	
	if (qbs_equal(qbs_ucase(context),qbs_new_txt("MUSIC"))||qbs_equal(qbs_ucase(context),qbs_new_txt("AUDIO"))||qbs_equal(qbs_ucase(context),qbs_new_txt("SOUND"))||qbs_equal(qbs_ucase(context),qbs_new_txt("SOUNDS"))||qbs_equal(qbs_ucase(context),qbs_new_txt("MY MUSIC"))){
		#ifdef QBNex_WINDOWS
			CHAR osPath[MAX_PATH];
			if(SUCCEEDED(SHGetFolderPathA(NULL,13,NULL,0,osPath))){ //Music
				return qbs_add(qbs_new_txt(osPath),qbs_new_txt("\\"));
            }
        #endif
    }
    
	if (qbs_equal(qbs_ucase(context),qbs_new_txt("PICTURE"))||qbs_equal(qbs_ucase(context),qbs_new_txt("PICTURES"))||qbs_equal(qbs_ucase(context),qbs_new_txt("IMAGE"))||qbs_equal(qbs_ucase(context),qbs_new_txt("IMAGES"))||qbs_equal(qbs_ucase(context),qbs_new_txt("MY PICTURES"))){
		#ifdef QBNex_WINDOWS
			CHAR osPath[MAX_PATH];
			if(SUCCEEDED(SHGetFolderPathA(NULL,39,NULL,0,osPath))){//Pictures
				return qbs_add(qbs_new_txt(osPath),qbs_new_txt("\\"));
            }
        #endif
    }
    
	if (qbs_equal(qbs_ucase(context),qbs_new_txt("DCIM"))||qbs_equal(qbs_ucase(context),qbs_new_txt("CAMERA"))||qbs_equal(qbs_ucase(context),qbs_new_txt("CAMERA ROLL"))||qbs_equal(qbs_ucase(context),qbs_new_txt("PHOTO"))||qbs_equal(qbs_ucase(context),qbs_new_txt("PHOTOS"))){
		#ifdef QBNex_WINDOWS
			CHAR osPath[MAX_PATH];
			if(SUCCEEDED(SHGetFolderPathA(NULL,39,NULL,0,osPath))){//Pictures
				return qbs_add(qbs_new_txt(osPath),qbs_new_txt("\\"));
            }
        #endif
    }
    
	if (qbs_equal(qbs_ucase(context),qbs_new_txt("MOVIE"))||qbs_equal(qbs_ucase(context),qbs_new_txt("MOVIES"))||qbs_equal(qbs_ucase(context),qbs_new_txt("VIDEO"))||qbs_equal(qbs_ucase(context),qbs_new_txt("VIDEOS"))||qbs_equal(qbs_ucase(context),qbs_new_txt("MY VIDEOS"))){
		#ifdef QBNex_WINDOWS
			CHAR osPath[MAX_PATH];
			if(SUCCEEDED(SHGetFolderPathA(NULL,14,NULL,0,osPath))){ //Videos
				return qbs_add(qbs_new_txt(osPath),qbs_new_txt("\\"));
            }
        #endif
    }
    
	if (qbs_equal(qbs_ucase(context),qbs_new_txt("DOWNLOAD"))||qbs_equal(qbs_ucase(context),qbs_new_txt("DOWNLOADS"))){
		#ifdef QBNex_WINDOWS
			CHAR osPath[MAX_PATH];
			if(SUCCEEDED(SHGetFolderPathA(NULL,0x0028,NULL,0,osPath))){//user folder
				//XP & SHGetFolderPathA do not support the concept of a Downloads folder, however it can be constructed
				mkdir((char*)((qbs_add(qbs_new_txt(osPath),qbs_new_txt_len("\\Downloads\0",11)))->chr));
				return qbs_add(qbs_new_txt(osPath),qbs_new_txt("\\Downloads\\"));
            }
        #endif
    }
    
	if (qbs_equal(qbs_ucase(context),qbs_new_txt("DESKTOP"))){
		#ifdef QBNex_WINDOWS
			CHAR osPath[MAX_PATH];
			if(SUCCEEDED(SHGetFolderPathA(NULL,0,NULL,0,osPath))){ //Desktop
				return qbs_add(qbs_new_txt(osPath),qbs_new_txt("\\"));
            }
        #endif
    }
    
	if (qbs_equal(qbs_ucase(context),qbs_new_txt("APPDATA"))||qbs_equal(qbs_ucase(context),qbs_new_txt("APPLICATION DATA"))||qbs_equal(qbs_ucase(context),qbs_new_txt("PROGRAM DATA"))||qbs_equal(qbs_ucase(context),qbs_new_txt("DATA"))){
		#ifdef QBNex_WINDOWS
			CHAR osPath[MAX_PATH];
			if(SUCCEEDED(SHGetFolderPathA(NULL,0x001a,NULL,0,osPath))){ //CSIDL_APPDATA (%APPDATA%)
				return qbs_add(qbs_new_txt(osPath),qbs_new_txt("\\"));
            }
        #endif
    }
    
	if (qbs_equal(qbs_ucase(context),qbs_new_txt("LOCALAPPDATA"))||qbs_equal(qbs_ucase(context),qbs_new_txt("LOCAL APPLICATION DATA"))||qbs_equal(qbs_ucase(context),qbs_new_txt("LOCAL PROGRAM DATA"))||qbs_equal(qbs_ucase(context),qbs_new_txt("LOCAL DATA"))){
		#ifdef QBNex_WINDOWS
			CHAR osPath[MAX_PATH];
			if(SUCCEEDED(SHGetFolderPathA(NULL,0x001c,NULL,0,osPath))){ //CSIDL_LOCAL_APPDATA (%LOCALAPPDATA%)
				return qbs_add(qbs_new_txt(osPath),qbs_new_txt("\\"));
            }
        #endif
    }
	
		if (qbs_equal(qbs_ucase(context),qbs_new_txt("PROGRAMFILES"))||qbs_equal(qbs_ucase(context),qbs_new_txt("PROGRAM FILES"))){
		#ifdef QBNex_WINDOWS
			CHAR osPath[MAX_PATH];
			if(SUCCEEDED(SHGetFolderPathA(NULL,0x0026,NULL,0,osPath))){ //CSIDL_PROGRAM_FILES (%PROGRAMFILES%)
				return qbs_add(qbs_new_txt(osPath),qbs_new_txt("\\"));
            }
        #endif
    }
	
	if (qbs_equal(qbs_ucase(context),qbs_new_txt("PROGRAMFILESX86"))||qbs_equal(qbs_ucase(context),qbs_new_txt("PROGRAMFILES X86"))||qbs_equal(qbs_ucase(context),qbs_new_txt("PROGRAM FILES X86"))||qbs_equal(qbs_ucase(context),qbs_new_txt("PROGRAM FILES 86"))||qbs_equal(qbs_ucase(context),qbs_new_txt("PROGRAM FILES (X86)"))||qbs_equal(qbs_ucase(context),qbs_new_txt("PROGRAMFILES (X86)"))||qbs_equal(qbs_ucase(context),qbs_new_txt("PROGRAM FILES(X86)"))){
		#ifdef QBNex_WINDOWS && _WIN64
			CHAR osPath[MAX_PATH];
			if(SUCCEEDED(SHGetFolderPathA(NULL,0x002a,NULL,0,osPath))){ //CSIDL_PROGRAM_FILES (%PROGRAMFILES(X86)%)
				return qbs_add(qbs_new_txt(osPath),qbs_new_txt("\\"));
            }
        #endif
    }
	
	if (qbs_equal(qbs_ucase(context),qbs_new_txt("TEMP"))||qbs_equal(qbs_ucase(context),qbs_new_txt("TEMP FILES"))){
		#ifdef QBNex_WINDOWS
			CHAR osPath[MAX_PATH+1];
			DWORD pathlen;
			pathlen = GetTempPathA(261, osPath); //%TEMP%
			char path[pathlen];
			memcpy(path, &osPath, pathlen);
			if (pathlen > 0){
				return qbs_new_txt(path);
			}
        #endif
    }
    
	//general fallback location
	#ifdef QBNex_WINDOWS
		CHAR osPath[MAX_PATH];
		if(SUCCEEDED(SHGetFolderPathA(NULL,0,NULL,0,osPath))){ //desktop
			return qbs_add(qbs_new_txt(osPath),qbs_new_txt("\\"));
        }
		return qbs_new_txt(".\\");//current location
        #else
		return qbs_new_txt("./");//current location
    #endif
}

extern void set_dynamic_info();
int main( int argc, char* argv[] ){
    
    #if defined(QBNex_LINUX) && defined(X11)
        XInitThreads();
    #endif
    
    static int32 i,i2,i3,i4;
    static uint8 c,c2,c3,c4;
    static int32 x,x2,x3,x4;
    static int32 y,y2,y3,y4;
    static int32 z,z2,z3,z4;
    static float f,f2,f3,f4;
    static uint8 *cp,*cp2,*cp3,*cp4;
    
    
    
    
    
    /********** Render State **********/
    render_state.dest=NULL;
    render_state.source=NULL;
    render_state.dest_handle=INVALID_HARDWARE_HANDLE;
    render_state.source_handle=INVALID_HARDWARE_HANDLE;
    render_state.view_mode=VIEW_MODE__UNKNOWN;
    render_state.use_alpha=ALPHA_MODE__UNKNOWN;
    render_state.depthbuffer_mode=DEPTHBUFFER_MODE__UNKNOWN;
    render_state.cull_mode=CULL_MODE__UNKNOWN;
    /********** Render State **********/
    
    
    
    
    for (i=0;i<=2;i++){
        display_frame[i].state=DISPLAY_FRAME_STATE__EMPTY;
        display_frame[i].order=0;
        display_frame[i].bgra=NULL;
        display_frame[i].w=0; display_frame[i].h=0;
        display_frame[i].bytes=0;
    }
    
    
    
    
    set_dynamic_info();
    if (ScreenResize){
        resize_snapback=0;
    }
    if (ScreenResizeScale){
        resize_auto=ScreenResizeScale;
    }
    
    
    //setup lists
    mouse_message_queue_handles=list_new(sizeof(mouse_message_queue_struct));
    special_handles=list_new(sizeof(special_handle_struct));
    stream_handles=list_new(sizeof(stream_struct));
    connection_handles=list_new(sizeof(connection_struct));
    
    hardware_img_handles=list_new_threadsafe(sizeof(hardware_img_struct));
    hardware_graphics_command_handles=list_new(sizeof(hardware_graphics_command_struct));
    
    //setup default mouse message queue
    mouse_message_queue_first=list_add(mouse_message_queue_handles);
    mouse_message_queue_default=mouse_message_queue_first;
    mouse_message_queue_struct *this_mouse_message_queue=(mouse_message_queue_struct*)list_get(mouse_message_queue_handles,mouse_message_queue_default);
    initialize_mouse_message_queue(this_mouse_message_queue,65535);
    
    snd_init();
    
    
    
    if (screen_hide_startup) screen_hide=1;
    
    #ifdef QBNex_WINDOWS
        if (console){
            std::vector<DWORD> plist(256,0);
            if (GetConsoleProcessList(plist.data(),256)==1){
                console_child=1;//only this program is using the console
            }
        }
    #endif
    
    
    
    
    
    onkey[1].keycode=59<<8;//F1-F10
    onkey[2].keycode=60<<8;
    onkey[3].keycode=61<<8;
    onkey[4].keycode=62<<8;
    onkey[5].keycode=63<<8;
    onkey[6].keycode=64<<8;
    onkey[7].keycode=65<<8;
    onkey[8].keycode=66<<8;
    onkey[9].keycode=67<<8;
    onkey[10].keycode=68<<8;
    onkey[11].keycode=72<<8;//up,left,right,down
    onkey[11].keycode_alternate=VK+QBVK_KP8;
    onkey[12].keycode=75<<8;
    onkey[12].keycode_alternate=VK+QBVK_KP4;
    onkey[13].keycode=77<<8;
    onkey[13].keycode_alternate=VK+QBVK_KP6;
    onkey[14].keycode=80<<8;
    onkey[14].keycode_alternate=VK+QBVK_KP2;
    onkey[30].keycode=133<<8;//F11,F12
    onkey[31].keycode=134<<8;
    
    //ON TIMER slot 0 is allocated by the Rust timer table (crates/qb-runtime/src/events.rs)
    
    fontwidth[8]=8; fontwidth[14]=8; fontwidth[16]=8;
    fontheight[8]=8; fontheight[14]=14; fontheight[16]=16;
    fontflags[8]=16; fontflags[14]=16; fontflags[16]=16;//monospace flag
    fontwidth[8+1]=8*2; fontwidth[14+1]=8*2; fontwidth[16+1]=8*2;
    fontheight[8+1]=8; fontheight[14+1]=14; fontheight[16+1]=16;
    fontflags[8+1]=16; fontflags[14+1]=16; fontflags[16+1]=16;//monospace flag
    
    memset(img,0,IMG_BUFFERSIZE*sizeof(img_struct));
    x=newimg();//reserve index 0
    img[x].valid=0;
    x=newimg();//reserve index 1
    img[x].valid=0;
    
    
    
    
    
    
    
    
    memset(&cpu,0,sizeof(cpu_struct));
    
    //uint8 *asmcodep=(uint8*)&asmcode[0];
    //memcpy(&cmem[0],asmcodep,sizeof(asmcode));
    reg8[0]=&cpu.al;
    reg8[1]=&cpu.cl;
    reg8[2]=&cpu.dl;
    reg8[3]=&cpu.bl;
    reg8[4]=&cpu.ah;
    reg8[5]=&cpu.ch;
    reg8[6]=&cpu.dh;
    reg8[7]=&cpu.bh;
    
    reg16[0]=&cpu.ax;
    reg16[1]=&cpu.cx;
    reg16[2]=&cpu.dx;
    reg16[3]=&cpu.bx;
    reg16[4]=&cpu.sp;
    reg16[5]=&cpu.bp;
    reg16[6]=&cpu.si;
    reg16[7]=&cpu.di;
    
    reg32[0]=&cpu.eax;
    reg32[1]=&cpu.ecx;
    reg32[2]=&cpu.edx;
    reg32[3]=&cpu.ebx;
    reg32[4]=&cpu.esp;
    reg32[5]=&cpu.ebp;
    reg32[6]=&cpu.esi;
    reg32[7]=&cpu.edi;
    
    segreg[0]=&cpu.es;
    segreg[1]=&cpu.cs;
    segreg[2]=&cpu.ss;
    segreg[3]=&cpu.ds;
    segreg[4]=&cpu.fs;
    segreg[5]=&cpu.gs;
    
    #ifdef QBNex_WINDOWS
        
        
        
        
        
        /*
            
            HANDLE WINAPI GetStdHandle(
            __in  DWORD nStdHandle
            );
            STD_INPUT_HANDLE
            (DWORD)-10
            
            
            
            The standard input device. Initially, this is the console input buffer, CONIN$.
            
            STD_OUTPUT_HANDLE
            (DWORD)-11
            
            
            
            The standard output device. Initially, this is the active console screen buffer, CONOUT$.
            
            STD_ERROR_HANDLE
            (DWORD)-12
            
            
            
            The standard error device. Initially, this is the active console screen buffer, CONOUT$.
            
            // BOOL WINAPI SetConsoleMode(
            //   __in  HANDLE hConsoleHandle,
            //   __in  DWORD dwMode
            // );
        */
        
        
        
    #endif
    

    
    
    port60h_event[0]=128+1;//simulate release of ESC
    
    
    if (int32 e=qb_mem_static_new_block(1048576)) error(e);//1MEG
    
    memset(&cmem[0],0,sizeof(cmem));
    
    
    
    
    
    
    
    
    
    
    memset(&keyon[0],0,sizeof(keyon));
    
    dblock=(ptrszint)&cmem+1280;//0:500h
    
    //define "nothing"
    cmem_sp-=8; nothingvalue=(uint64*)(dblock+cmem_sp);
    *nothingvalue=0;
    nothingstring=qbs_new_cmem(0,0);
    singlespace=qbs_new_cmem(1,0);
    singlespace->chr[0]=32;
    
    //store _CWD$ for recall using _STARTDIR$ in startDir
    startDir=qbs_new(0,0);
    qbs_set(startDir,func__cwd());
    
    //switch to directory of this EXE file
    //http://stackoverflow.com/questions/1023306/finding-current-executables-path-without-proc-self-exe
    #if defined(QBNex_WINDOWS) && !defined(QBNex_MICROSOFT)
        static std::vector<char> exepath_storage(65536,0);
        char *exepath=exepath_storage.data();
        GetModuleFileName(NULL,exepath,65536);
        i=strlen(exepath);
        for (i2=i-1;i2>=0;i2--){
            x=exepath[i2];
            if ((x==92)||(x==47)||(x==58)){
                if (x==58) exepath[i2+1]=0; else exepath[i2]=0;
                break;
            }
        }
        chdir(exepath);
        #elif defined(QBNex_LINUX)
        {
            char pathbuf[65536];
            memset(pathbuf, 0, sizeof(pathbuf));
            readlink("/proc/self/exe", pathbuf, 65535);
            chdir(dirname(pathbuf));
        }
        #elif defined(QBNex_MACOSX)
        {
            char pathbuf[65536];
            uint32_t pathbufsize = sizeof(pathbuf);
            _NSGetExecutablePath(pathbuf, &pathbufsize);                        
            chdir(dirname(pathbuf));
        }
    #endif
    
    rootDir=qbs_new(0,0);
    qbs_set(rootDir,func__cwd());
    
    unknown_opcode_mess=qbs_new(0,0);
    qbs_set(unknown_opcode_mess,qbs_new_txt_len("Unknown Opcode (  )\0",20));
    
    i=argc;
    if (i>1){
        //calculate required size of COMMAND$ string
        i2=0;
        for (i=1;i<argc;i++){
            i2+=strlen(argv[i]);
            if (i!=1) i2++;//for a space
        }
        //create COMMAND$ string
        func_command_str=qbs_new(i2,0);
        //build COMMAND$ string
        i3=0;
        for (i=1;i<argc;i++){
            if (i!=1){func_command_str->chr[i3]=32; i3++;}
            memcpy(&func_command_str->chr[i3],argv[i],strlen(argv[i])); i3+=strlen(argv[i]);
        }
        }else{
        func_command_str=qbs_new(0,0);
    }
    
    func_command_count = argc;
    func_command_array = argv;
    
    
    // TIMER reference: tick count and local time of day, taken right after the
    // wall clock's second ticks over (crates/qb-runtime/src/systime.rs).
    qb_time_init(&clock_firsttimervalue,&qbnex_firsttimervalue);
    
    //init truetype .ttf/.fon font library
    
    #ifdef QBNex_WINDOWS 
        //for caps lock, use the state of the lock (=1)
        //for held keys check against (=-127)
        if (GetKeyState(VK_SCROLL)&1) keyheld_add(QBK+QBK_SCROLL_LOCK_MODE);
        if (GetKeyState(VK_SCROLL)<0){bindkey=QBVK_SCROLLOCK; keydown_vk(VK+QBVK_SCROLLOCK);}
        if (GetKeyState(VK_LSHIFT)<0){bindkey=QBVK_LSHIFT; keydown_vk(VK+QBVK_LSHIFT);}
        if (GetKeyState(VK_RSHIFT)<0){bindkey=QBVK_RSHIFT; keydown_vk(VK+QBVK_RSHIFT);}
        if (GetKeyState(VK_LCONTROL)<0){bindkey=QBVK_LCTRL; keydown_vk(VK+QBVK_LCTRL);}
        if (GetKeyState(VK_RCONTROL)<0){bindkey=QBVK_RCTRL; keydown_vk(VK+QBVK_RCTRL);}
        if (GetKeyState(VK_LMENU)<0){bindkey=QBVK_LALT; keydown_vk(VK+QBVK_LALT);}
        if (GetKeyState(VK_RMENU)<0){bindkey=QBVK_RALT; keydown_vk(VK+QBVK_RALT);}
        if (GetKeyState(VK_CAPITAL)&1){bindkey=QBVK_CAPSLOCK; keydown_vk(VK+QBVK_CAPSLOCK);}
        if (GetKeyState(VK_NUMLOCK)&1){bindkey=QBVK_NUMLOCK; keydown_vk(VK+QBVK_NUMLOCK);}
        update_shift_state();
        qb_keyhit_clear();//skip hitkey events generated by above code
    #endif
    
    
    //init fake keyb. cyclic buffer
    cmem[0x41a]=30; cmem[0x41b]=0; //head
    cmem[0x41c]=30; cmem[0x41d]=0; //tail
    
    ifstream fh;
    
    //default palettes, SCREEN 10 pseudo colors and 8x8/8x16 fonts: Rust statics (qb-runtime gfx/data.rs)
    
    qbg_screen(0,NULL,NULL,NULL,NULL,1);
    width8050switch=1;//reaffirm switch reset by above command
    
    if (console){
        console_image=func__newimage(80,25,0,0);
        i=-console_image;
        img[i].console=1;
    }
    
    
    //int pthread_create(pthread_t *thread, const pthread_attr_t *attr,
    //                   void *(*start_routine) (void *), void *arg);
    
    
    //create icon images
    image_qbicon16_handle=func__newimage(image_qbicon16_w,image_qbicon16_h,32,1);
    memcpy(img[-image_qbicon16_handle].offset32,&image_qbicon16[0],image_qbicon16_w*image_qbicon16_h*4);
    image_qbicon32_handle=func__newimage(image_qbicon32_w,image_qbicon32_h,32,1);
    memcpy(img[-image_qbicon32_handle].offset32,&image_qbicon32[0],image_qbicon32_w*image_qbicon32_h*4);
    
    
    
    
    
    
    //setup default _DEVICE(s)
    i=0;
    
    //keyboard
    i++;
    devices[i].type=DEVICETYPE_KEYBOARD;
    devices[i].name="[KEYBOARD][BUTTON]";
    devices[i].lastbutton=512;
    devices[i].description="Keyboard";
    setupDevice(&devices[i]);
    
    //mouse
    i++;
    devices[i].type=DEVICETYPE_MOUSE;
    devices[i].name="[MOUSE][BUTTON][AXIS][WHEEL]";
    devices[i].lastbutton=3;
    devices[i].lastaxis=2;
    devices[i].lastwheel=3;
    devices[i].description="Mouse";
    setupDevice(&devices[i]);
    
    device_last=i;
    
    
    #ifdef DEPENDENCY_DEVICEINPUT
        QBNex_GAMEPAD_INIT();
    #endif
    
    qb_start_detached_thread([](){
        QBMAIN(NULL);
    });
    
    qb_start_detached_thread([](){
        qb_timer_thread();
    });
    
    
    
    
    
    
    
    
    
    
    
    
    
    
    /*
        GLenum err = glewInit();
        if (GLEW_OK != err)
        {
        exit(0);
        }
        //fprintf(stdout, "Status: Using GLEW %s\n", glewGetString(GLEW_VERSION));
        
        
        GLenum err = glewInit();
        if (GLEW_OK != err)
        {
        qbs_print(qbs_str((int32)err),1);
        
        qbs_print(qbs_new_txt((char*)glewGetString(err)),1);
        
        //error(70);
        //exit(0);
        }
    */
    
    lock_display_required=1;
    
    #ifndef QBNex_GUI
        //normally MAIN_LOOP() is launched in a separate thread to reserve the primary thread for GLUT
        //that is not required, so run MAIN_LOOP() in our primary thread
        MAIN_LOOP();
        exit(0);
    #endif    
    
    qb_start_detached_thread([](){
        MAIN_LOOP();
    });
    
    if (!screen_hide) create_window=1;

    //R8: winit event loop + softbuffer presenter (crates/qb-runtime/src/window.rs)
    window_main();
    while (1) Sleep(1000);//headless: MAIN_LOOP ends the program with exit()
    
}



//###################### Main Loop ####################
#ifdef QBNex_WINDOWS
    void MAIN_LOOP_WINDOWS(void *unused){
        MAIN_LOOP();
        return;
    }
    #else
    void *MAIN_LOOP_LINUX(void *unused){
        MAIN_LOOP();
        return NULL;
    }
#endif
void MAIN_LOOP(){
    
    int32 update=0;//0=update input,1=update display
    
    main_loop:
    
    #ifdef DEPENDENCY_DEVICEINPUT
        #ifndef QBNex_MACOSX 
            QBNex_GAMEPAD_POLL();
        #endif
    #endif        
    
    if (lock_mainloop==1){
        lock_mainloop=2;
        while (lock_mainloop==2) Sleep(1);
    }
    
    if (exit_value){
        if (!exit_blocked) goto end_program;
    }
    
    
    //update timer bytes in cmem
    static uint32 cmem_ticks;
    static double cmem_ticks_double;
    
    cmem_ticks=GetTicks();
    cmem_ticks-=clock_firsttimervalue;
    cmem_ticks+=qbnex_firsttimervalue;
    //make timer value loop after midnight
    //note: there are 86400000 milliseconds in 24hrs(1 day)
    cmem_ticks%=86400000;
    cmem_ticks=((double)cmem_ticks)*0.0182;
    cmem[0x46c]=cmem_ticks&255;
    cmem[0x46d]=(cmem_ticks>>8)&255;
    cmem[0x46e]=(cmem_ticks>>16)&255;
    //note: a discrepancy exists of unknown cause
    
    if (shell_call_in_progress){
        if (shell_call_in_progress!=-1){
            shell_call_in_progress=-1;
            goto update_display_only;
        }
        Sleep(64);
        goto main_loop;
    }
    
    Sleep(15);
    vertical_retrace_happened=1; vertical_retrace_in_progress=1;
    Sleep(1);
    
    if (close_program){
        lock_mainloop=2;//report mainloop as locked so that any process waiting for a successful lock can continue
        goto end_program;
    }
    
    snd_mainloop();
    
    


update^=1;//toggle update

if (!lprint){//not currently performing an LPRINT operation
    lprint_locked=1;
    if (lprint_buffered){
        if (fabs(func_timer(0.001,1)-lprint_last)>=10.0){//10 seconds elapsed since last LPRINT operation
            sub__printimage(lprint_image);
            lprint_buffered=0;
            static int32 old_dest;
            old_dest=func__dest();
            sub__dest(lprint_image);
            sub_cls(NULL,15,2);
            sub__dest(old_dest);
        }
    }
    lprint_locked=0;
}



//note: this mainloop loops with breaks of 16ms, display is toggled every 2nd loop
//update display?
if (update==1){
    update_display_only:
    if (autodisplay) display();//occurs every 32ms or 31.25 times per second
    else if (os_resize_event||full_screen_set!=-1) present_frame();
    frame++;//~32 fps
}//update==1

vertical_retrace_in_progress=0;

if (shell_call_in_progress) goto main_loop;

if (update==0){
    
    static int32 scancode;
    static const uint8 QBVK_2_scancode[] = {
        0,0,0,0,0,0,0,0,14,15,0,0,0,28,0,0,0,0,0,0,0,0,0,0,0,0,0,1,0,0,0,0,57,0,0,0,0,0,0,40,0,0,0,0,51,12,52,53,11,2,3,4,5,6,7,8,9,10,0,39,0,13,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,26,43,27,0,0,41,30,48,46,32,18,33,34,35,23,36,37,38,50,49,24,25,16,19,31,20,22,47,17,45,21,44,0,0,0,0,83,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,82,79,80,81,75,76,77,71,72,73,83,53,55,74,78,28,0,72,80,77,75,82,71,79,73,81,59,60,61,62,63,64,65,66,67,68,133,134,0,0,0,0,0,0,69,58,70,54,42,29,29,56,56,0,0,91,92,0,0,0,0,55,197,93,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
        0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
        0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    };
    
    qb_input_lock();
    #ifdef QBNex_WINDOWS
        //manage important external keyboard lock/state changes
        if ((GetKeyState(VK_SCROLL)&1)!=keyheld(QBK+QBK_SCROLL_LOCK_MODE)){
            if (keyheld(QBK+QBK_SCROLL_LOCK_MODE)){
                keyheld_remove(QBK+QBK_SCROLL_LOCK_MODE);
                }else{
                keyheld_add(QBK+QBK_SCROLL_LOCK_MODE);
            }
            update_shift_state();
        }
        if ((GetKeyState(VK_CAPITAL)&1)!=keyheld(VK+QBVK_CAPSLOCK)){
            if (keyheld(VK+QBVK_CAPSLOCK)){
                bindkey=QBVK_CAPSLOCK; keyup_vk(VK+QBVK_CAPSLOCK);
                }else{
                bindkey=QBVK_CAPSLOCK; keydown_vk(VK+QBVK_CAPSLOCK);
            }
            update_shift_state();
        }
        if ((GetKeyState(VK_NUMLOCK)&1)!=keyheld(VK+QBVK_NUMLOCK)){
            if (keyheld(VK+QBVK_NUMLOCK)){
                bindkey=QBVK_NUMLOCK; keyup_vk(VK+QBVK_NUMLOCK);
                }else{
                bindkey=QBVK_NUMLOCK; keydown_vk(VK+QBVK_NUMLOCK);
            }
            update_shift_state();
        }
    #endif
    qb_input_unlock();

    if (shell_call_in_progress) goto main_loop;
    
}//update==0

goto main_loop;

end_program:
stop_program=1;
qbevent=1;
while (exit_ok!=3) Sleep(16);

if (lprint_buffered){
    sub__printimage(lprint_image);//print any pending content
}

//close all open files
sub_close(NULL,0);

//shutdown device interface
#ifdef DEPENDENCY_DEVICEINPUT
    QBNex_GAMEPAD_SHUTDOWN();
#endif

snd_un_init();

exit(exit_code);
}









//used to preserve the previous frame's content for comparison/reuse purposes
static std::vector<uint8> pixeldata_storage(1,0);
uint8 *pixeldata=pixeldata_storage.data();
int32 pixeldatasize=static_cast<int32>(pixeldata_storage.size());
uint32 paldata[256];

//note: temporarily swapping a source palette is far more effecient than converting the resulting image pixels
void swap_paldata_BGRA_with_RGBA(){
    static uint32 col;
    static uint32 *pos;
    pos=(uint32*)&paldata[0];
    static int32 pixels;
    pixels=256;
    while(pixels--){
        col=*pos;
        *pos++= (col&0xFF00FF00) | ((col & 0xFF0000) >> 16) | ((col & 0x0000FF) << 16);
    }
}

//display updates the visual page onto the visible window/monitor
void display(){
    
    
    
    //general variables
    static int32 i,i2,i3,i4;
    static uint8 c,c2,c3,c4;
    static int32 x,x2,x3,x4;
    static int32 y,y2,y3,y4;
    static int32 z,z2,z3,z4;
    static float f,f2,f3,f4;
    static uint8 *cp,*cp2,*cp3,*cp4;
    uint32 *pixel;
    
    static uint8 BGRA_to_RGBA;//set to 1 to invert the output to RGBA
    BGRA_to_RGBA=0;//default is 0 but 1 is fun    
    
    
    
    if (lock_display==1){lock_display=2; Sleep(0);}

    if (screen_hide) {display_called=1; return;}

    std::lock_guard<std::recursive_mutex> present_guard(present_mutex);

    if (lock_display==0){
        
        
        
        //Identify which display_frame to build
        static int32 frame_i;
        frame_i=-1;
        //use LOWEST order ready frame
        static int64 lowest_order;
        lowest_order=display_frame_order_next;
        for (i=0;i<=2;i++){
            if (display_frame[i].order<lowest_order&&display_frame[i].state==DISPLAY_FRAME_STATE__READY){
                lowest_order=display_frame[i].order;
                frame_i=i;
            }
        }
        //or preferably an unused frame if possible (note: this check happens 2nd for threading reasons)
        for (i=0;i<=2;i++){
            if (display_frame[i].state==DISPLAY_FRAME_STATE__EMPTY){
                frame_i=i;    
            }
        }
        if (frame_i==-1){
            alert("Software frame buffer: Failed to find available frame");
            return;
        }
        display_frame[frame_i].state=DISPLAY_FRAME_STATE__BUILDING;
        display_frame[frame_i].order=display_frame_order_next++;
        
        
        
        
        //validate display_page
        if (!display_page) goto display_page_invalid;
        
        
        
        //check what is possible in full screen
        x=display_page->width; y=display_page->height;
        
        if (display_page->compatible_mode==0){
            x=display_page->width*fontwidth[display_page->font]; y=display_page->height*fontheight[display_page->font];
        }
        
        //check for y-stretch flag?
        if (x<=512&&y<=384){
            x*=2; y*=2;
        }
        
        static int32 mode_square,mode_stretch;
        
        //find best fullscreen mode(s) (eg. square/"1:1", stretched)
        mode_square=-1;
        mode_stretch=-1;
        
        x=display_page->width; y=display_page->height;
        if (display_page->compatible_mode==0){
            x=display_page->width*fontwidth[display_page->font]; y=display_page->height*fontheight[display_page->font];
        }
        x_monitor=x; y_monitor=y;
        
        z=0; //?
        
        conversion_required=0;
        pixel=display_surface_offset;//<-will be made obselete
        
        if (!display_page->compatible_mode){//text
            
            static int32 show_flashing_last=0;
            static int32 show_cursor_last=0;
            static int32 check_last;
            static uint8 *cp,*cp2,*cp_last;
            static uint32 *lp;
            static int32 cx,cy;
            static int32 cx_last=-1,cy_last=-1;
            static int32 show_cursor;
            static int32 show_flashing;
            static uint8 chr,col,chr_last,col_last;
            static int32 qbg_y_offset;
            static const std::string *cell_utf8;
            static int32 rendered_utf8;
            static int32 cell_index;
            static int32 cell_span;
            
            static int32 f,f_pitch,f_width,f_height;//font info
            f=display_page->font; f_width=fontwidth[f]; f_height=fontheight[f];
            
            static int32 content_changed;
            
            
            check_last=screen_last_valid;//If set, modify the previous pixelbuffer's contents
            content_changed=0;
            
            //Realloc pixel-buffer if necessary
            i=display_page->width*display_page->height*2;
            if (screen_last_size!=i){
                screen_last_storage.resize(i,0);
                screen_last=screen_last_storage.data();
                screen_last_size=static_cast<uint32>(screen_last_storage.size());
                check_last=0;
            }
            
            if (displayorder_screen==0 && check_last==1){
                //a valid frame of the correct dimensions exists and we are not required to display software content
                goto no_new_frame;
            }
            
            //Check/Prepare palette-buffer
            if (!check_last){
                //set pal_last (no prev pal was avilable to compare to)
                memcpy(&paldata,display_page->pal,256*4);
                }else{
                //if palette has changed, update paldata and draw all characters
                if (memcmp(&paldata[0],display_page->pal,256*4)){
                    //Different palette
                    memcpy(&paldata[0],display_page->pal,256*4);
                    check_last=0;
                }
            }
            if (qb_text_page_has_utf8_content(display_page_index)) check_last=0;
            
            //Check/Prepare content
            if (check_last){
                //i=display_frame_end;
                if (memcmp(screen_last,display_page->offset,screen_last_size)){
                    //Different content
                    content_changed=1;
                }
            }
            
            //Note: frame is a global variable incremented ~32 times per second [2013]
            if (frame&8) show_cursor=1; else show_cursor=0;//[2013]halved cursor blink rate from 8 changes p/sec -> 4 changes p/sec
            if (frame&8) show_flashing=1; else show_flashing=0;
            
            //calculate cursor position (base 0)
            cx=display_page->cursor_x-1; cy=display_page->cursor_y-1;
            if (display_page->holding_cursor==2){//special case
                if (cy<(display_page->height-1)){cy++; cx=0;}
            }
            
            if (check_last){
                if (show_flashing!=show_flashing_last) content_changed=1;
                if (show_cursor!=show_cursor_last) content_changed=1;
                if ((cx!=cx_last)||(cy!=cy_last)) content_changed=1;
            }
            
            if (!check_last) content_changed=1;
            
            if (!content_changed){
                //No content has changed, so skip the generation & display of this frame
                goto no_new_frame;
            }
            
            static int64 last_frame_i=0;
            
            //################################ Setup new frame ################################
            {
                static int32 new_size_bytes;
                new_size_bytes=x_monitor*y_monitor*4;
                if (new_size_bytes>display_frame[frame_i].bytes){
                    display_frame[frame_i].bgra_storage.resize(static_cast<size_t>(new_size_bytes/4),0);
                    display_frame[frame_i].bgra=display_frame[frame_i].bgra_storage.data();
                    display_frame[frame_i].bytes=new_size_bytes;
                }
                display_frame[frame_i].w=x_monitor; display_frame[frame_i].h=y_monitor;
            }
            
            display_surface_offset=display_frame[frame_i].bgra;
            
            //If a compare & update changes method will be used copy the previous content to the new buffer
            
            if (check_last){
                //find the most recently published page to compare with
                //(the most recent READY or DISPLAYING page)
                static int64 highest_order;
                highest_order=0;
                i2=-1;
                for (i3=0;i3<=2;i3++){
                    if ((display_frame[i3].state==DISPLAY_FRAME_STATE__DISPLAYING||
                    display_frame[i3].state==DISPLAY_FRAME_STATE__READY)
                    &&display_frame[i3].order>highest_order){
                        highest_order=display_frame[i3].order;
                        i2=i3;
                    }
                } 
                if (i2!=-1){
                    memcpy(display_frame[frame_i].bgra,display_frame[i2].bgra,display_frame[frame_i].w*display_frame[frame_i].h*4);
                    }else{
                    alert("Text Screen Update: Failed to locate previous frame's data for comparison"); 
                    check_last=0;//never occurs, safe-guard only
                }
            }
            
            qbg_y_offset=0;//the screen base offset
            cp=display_page->offset;//read from
            cp_last=screen_last;//written to for future comparisons
            std::vector<std::string> *text_utf8_cells=NULL;
            std::vector<uint8> *text_utf8_cell_state=NULL;
            if (display_page_index >= 0 && display_page_index < qb_text_utf8_cells.size() && !qb_text_utf8_cells[display_page_index].empty()){
                text_utf8_cells=&qb_text_utf8_cells[display_page_index];
                text_utf8_cell_state=&qb_text_utf8_cell_state[display_page_index];
            }
            
            
            if (BGRA_to_RGBA) swap_paldata_BGRA_with_RGBA();
            
            //outer loop
            y2=0;
            for (y=0;y<display_page->height;y++){
                x2=0;
                for (x=0;x<display_page->width;x++){
                    
                    chr=*cp; cp++; col=*cp; cp++;
                    
                    //can be skipped?
                    chr_last=*cp_last; cp_last++; col_last=*cp_last; cp_last++;
                    
                    if (check_last){
                        if (chr==chr_last){//same character
                            if (col==col_last){//same colours
                                if (col&128) if (show_flashing!=show_flashing_last) goto cantskip;//same flash
                                if (x==cx) if (y==cy) if (show_cursor!=show_cursor_last) goto cantskip;//same cursor
                                if (x==cx_last){ if (y==cy_last){
                                    if ((cx!=cx_last)||(cy!=cy_last)) goto cantskip;//fixup old cursor's location
                                }}
                                goto skip;
                            }}}
                            cantskip:
                            cp_last-=2; *cp_last=chr; cp_last++; *cp_last=col; cp_last++;
                            cell_utf8=NULL;
                            cell_span=1;
                            if (text_utf8_cells){
                                cell_index=y*display_page->width+x;
                                cell_span=qb_text_cell_span(display_page_index,cell_index);
                                if (cell_span == 0) goto skip;
                                if (cell_index >= 0 && cell_index < static_cast<int32>(text_utf8_cells->size()) && !(*text_utf8_cells)[cell_index].empty()){
                                    cell_utf8=&((*text_utf8_cells)[cell_index]);
                                }
                            }
                            
                            //set cp2 to the character's data
                            z2=0;//double-width if set
                            
                            if (f>=32){//custom font
                                
                                static uint32 chr_utf32;
                                chr_utf32=codepage437_to_unicode16[chr];
                                
                                static uint8 *rt_data_last=NULL;
                                static int32 render_option;
                                static int32 ok;
                                static uint8 *rt_data;
                                static int32 rt_w,rt_h,rt_pre_x,rt_post_x;
                                render_option=1;
                                if (rt_data_last) free(rt_data_last);
                                ok=FontRenderTextUTF32(font[f],&chr_utf32,1,render_option,
                                &rt_data,&rt_w,&rt_h,&rt_pre_x,&rt_post_x);
                                rt_data_last=rt_data;
                                cp2=rt_data;
                                f_pitch=0;
                                
                                }else{//default font
                                f_pitch=0;
                                if (f==8) cp2=&charset8x8[chr][0][0];
                                if (f==14) cp2=&charset8x16[chr][1][0];
                                if (f==16) cp2=&charset8x16[chr][0][0];
                                if (f==(8+1)) {cp2=&charset8x8[chr][0][0]; z2=1;}
                                if (f==(14+1)) {cp2=&charset8x16[chr][1][0]; z2=1;}
                                if (f==(16+1)) {cp2=&charset8x16[chr][0][0]; z2=1;}
                            }
                            c=col&0xF;//foreground col
                            if (H3C0_blink_enable) {
                                c2=(col>>4)&7;//background col
                                c3=col>>7;//flashing?
                                } else {
                                c2=(col>>4);//background col
                            }
                            if (c3&&show_flashing && H3C0_blink_enable) c=c2;
                            i2=paldata[c];
                            i3=paldata[c2];
                            rendered_utf8=0;
                            if (cell_utf8){
                                #ifdef QBNex_WINDOWS
                                    rendered_utf8=qb_render_text_cell_windows(*cell_utf8,f_width*cell_span,f_height,i2,i3,display_surface_offset+qbg_y_offset+y2*x_monitor+x2,x_monitor);
                                    #else
                                    rendered_utf8=qb_render_text_cell_freetype(*cell_utf8,f_width*cell_span,f_height,i2,i3,display_surface_offset+qbg_y_offset+y2*x_monitor+x2,x_monitor);
                                #endif
                            }
                            if (!rendered_utf8){
                                lp=display_surface_offset+qbg_y_offset+y2*x_monitor+x2;
                                z=x_monitor-fontwidth[display_page->font];
                                
                                //inner loop
                                for (y3=0;y3<f_height;y3++){
                                    for (x3=0;x3<f_width;x3++){
                                        if (*cp2) *lp=i2; else *lp=i3;
                                        if (z2){
                                            if (x3&z2) cp2++;
                                            }else{
                                            cp2++;
                                        }
                                        lp++;
                                    }
                                    lp+=z;
                                    cp2+=f_pitch;
                                }//y3,x3
                            }
                            
                            //draw cursor
                            if (display_page->cursor_show&&show_cursor&&(cx==x)&&(cy==y)){
                                static int32 v1,v2;
                                static uint8 from_bottom;//bottom is the 2nd bottom scanline in width ?x25
                                static uint8 half_cursor;//if set, overrides all following values
                                static uint8 size;//if 0, no cursor is drawn, if 255, from begin to bottom
                                static uint8 begin;//only relevant if from_bottom was not specified
                                v1=display_page->cursor_firstvalue;
                                v2=display_page->cursor_lastvalue;
                                from_bottom=0;
                                half_cursor=0;
                                size=0;
                                begin=0;
                                //RULE: IF V2=0, NOTHING (UNLESS V1=0)
                                if (v2==0){
                                    if (v1==0){size=1; goto cursor_created;}
                                    goto nocursor;//no cursor!
                                }
                                //RULE: IF V2<V1, FROM V2 TO BOTTOM
                                if (v2<v1){begin=v2; size=255; goto cursor_created;}
                                //RULE: IF V1=V2, SINGLE SCANLINE AT V1 (OR BOTTOM IF V1>=4)
                                if (v1==v2){
                                    if (v1<=3){begin=v1; size=1; goto cursor_created;}
                                    from_bottom=1; size=1; goto cursor_created;
                                }
                                //NOTE: V2 MUST BE LARGER THAN V1!
                                //RULE: IF V1>=3, CALC. DIFFERENCE BETWEEN V1 & V2
                                //                IF DIFF=1, 2 SCANLINES AT BOTTOM
                                //                IF DIFF=2, 3 SCANLINES AT BOTTOM
                                //                OTHERWISE HALF CURSOR
                                if (v1>=3){
                                    if ((v2-v1)==1){from_bottom=1; size=2; goto cursor_created;}
                                    if ((v2-v1)==2){from_bottom=1; size=3; goto cursor_created;}
                                    half_cursor=1; goto cursor_created;
                                }
                                //RULE: IF V1<=1, IF V2<=3 FROM V1 TO V3 ELSE FROM V1 TO BOTTOM
                                if (v1<=1){
                                    if (v2<=3){begin=v1;size=v2-v1+1; goto cursor_created;} 
                                    begin=v1;size=255; goto cursor_created;
                                }
                                //RULE: IF V1=2, IF V2=3, 2 TO 3
                                //               IF V2=4, 3 SCANLINES AT BOTTOM
                                //               IF V2>=5, FROM 2 TO BOTTOM
                                //(assume V1=2)
                                if (v2==3){begin=2;size=2; goto cursor_created;}
                                if (v2==4){from_bottom=1; size=3; goto cursor_created;}
                                begin=2;size=255;
                                cursor_created:
                                static int32 cw,ch;
                                cw=fontwidth[display_page->font]; ch=fontheight[display_page->font];
                                if (half_cursor){
                                    //half cursor
                                    y3=ch-1;
                                    size=ch/2;
                                    c=col&0xF;//foreground col
                                    i2=paldata[c];
                                    draw_half_curs:
                                    lp=display_surface_offset+qbg_y_offset+(y2+y3)*x_monitor+x2;
                                    for (x3=0;x3<cw;x3++){
                                        *lp=i2;
                                        lp++;
                                    }
                                    y3--;
                                    size--;
                                    if (size) goto draw_half_curs;
                                    }else{
                                    if (from_bottom){
                                        //cursor from bottom
                                        y3=ch-1;
                                        if (y3==15) y3=14;//leave bottom line blank in 8x16 char set
                                        c=col&0xF;//foreground col
                                        i2=paldata[c];
                                        draw_curs_from_bottom:
                                        lp=display_surface_offset+qbg_y_offset+(y2+y3)*x_monitor+x2;
                                        for (x3=0;x3<cw;x3++){
                                            *lp=i2;
                                            lp++;
                                        }
                                        y3--;
                                        size--;
                                        if (size) goto draw_curs_from_bottom;
                                        }else{
                                        //cursor from begin using size
                                        if (begin<ch){
                                            y3=begin;
                                            c=col&0xF;//foreground col
                                            i2=paldata[c];
                                            if (size==255) size=ch-begin;
                                            draw_curs_from_begin:
                                            lp=display_surface_offset+qbg_y_offset+(y2+y3)*x_monitor+x2;
                                            for (x3=0;x3<cw;x3++){
                                                *lp=i2;
                                                lp++;
                                            }
                                            y3++;
                                            size--;
                                            if (size) goto draw_curs_from_begin;
                                        }
                                    }
                                }
                            }//draw cursor?
                            nocursor:
                            
                            //outer loop
                            skip:
                            x2=x2+fontwidth[display_page->font];
                }
                y2=y2+fontheight[display_page->font];
                
            }
            
            show_flashing_last=show_flashing;
            show_cursor_last=show_cursor;
            cx_last=cx;
            cy_last=cy;
            screen_last_valid=1;
            
            if (BGRA_to_RGBA) swap_paldata_BGRA_with_RGBA();
            
            /*
                //backup for reuse in next frame
                i=display_frame[frame_i].w*display_frame[frame_i].h*4;
                if (i!=pixeldatasize){
                free(pixeldata);
                pixeldata=(uint8*)malloc(i);
                pixeldatasize=i;
                }
                memcpy(pixeldata,display_frame[frame_i].bgra,i);
            */
            
            last_frame_i=frame_i;
            
            goto screen_refreshed;
            
        }//text
        
        
        
        
        
        
        
        
        
        
        
        
        
        
        
        
        
        
        
        
        
        
        
        if (display_page->bits_per_pixel==32){
            
            //note: as software->hardware should be avoided at all costs, pixeldata is
            //      still backed up for comparison purposes because in the very likely
            //      event the data has not changed there is no point generating a 
            //      new hardware surface from the software frame when the old hardware surface 
            //      can be reused. It also saves on BGRA->RGBA conversion on some platforms.
            
            if (!BGRA_to_RGBA){
                //find the most recently published page to compare with
                //(the most recent READY or DISPLAYING page)
                static int64 highest_order;
                highest_order=0;
                i2=-1;
                for (i3=0;i3<=2;i3++){
                    if ((display_frame[i3].state==DISPLAY_FRAME_STATE__DISPLAYING||
                    display_frame[i3].state==DISPLAY_FRAME_STATE__READY)
                    &&display_frame[i3].order>highest_order){
                        highest_order=display_frame[i3].order;
                        i2=i3;
                    }
                } 
                if (force_display_update) goto update_display32b; //force update
                if (i2!=-1){  
                    if (!screen_last_valid) goto update_display32b; //force update because of mode change?
                    i=display_page->width*display_page->height*4;
                    if (i!=(display_frame[i2].w*display_frame[i2].h*4)) goto update_display32b;
                    
                    if (displayorder_screen==0){
                        //a valid frame of the correct dimensions exists and we are not required to display software content
                        goto no_new_frame;
                    }
                    
                    if (memcmp(display_frame[i2].bgra,display_page->offset,i)) goto update_display32b;
                    goto no_new_frame;//no need to update display
                }
                update_display32b:;
                }else{
                
                //BGRA_to_RGBA
                i=display_page->width*display_page->height*4;
                if (i!=pixeldatasize){
                    pixeldata_storage.resize(i,0);
                    pixeldata=pixeldata_storage.data();
                    pixeldatasize=static_cast<int32>(pixeldata_storage.size());
                    goto update_display32;
                }
                if (force_display_update) goto update_display32; //force update
                
                if (displayorder_screen==0){
                    //a valid frame of the correct dimensions exists and we are not required to display software content
                    goto no_new_frame;
                }
                
                if (memcmp(pixeldata,display_page->offset,i)) goto update_display32;
                if (!screen_last_valid) goto update_display32; //force update because of mode change?
                goto no_new_frame;//no need to update display
                update_display32:
                memcpy(pixeldata,display_page->offset,i);
            }
            
            
            //################################ Setup new frame ################################
            {
                static int32 new_size_bytes;
                new_size_bytes=x_monitor*y_monitor*4;
                if (new_size_bytes>display_frame[frame_i].bytes){
                    display_frame[frame_i].bgra_storage.resize(static_cast<size_t>(new_size_bytes/4),0);
                    display_frame[frame_i].bgra=display_frame[frame_i].bgra_storage.data();
                    display_frame[frame_i].bytes=new_size_bytes;
                }
                display_frame[frame_i].w=x_monitor; display_frame[frame_i].h=y_monitor;
            }
            
            if (!BGRA_to_RGBA){
                memcpy(display_frame[frame_i].bgra,display_page->offset,display_frame[frame_i].w*display_frame[frame_i].h*4);
                }else{
                static uint32 col;
                static uint32 *src_pos;
                static uint32 *dst_pos;
                src_pos=(uint32*)pixeldata;
                dst_pos=display_frame[frame_i].bgra;
                static int32 pixels;
                pixels=display_frame[frame_i].w*display_frame[frame_i].h;
                if (pixels>0){
                    while(pixels--){
                        col=*src_pos++;
                        *dst_pos++= (col&0xFF00FF00) | ((col & 0xFF0000) >> 16) | ((col & 0x0000FF) << 16);
                    }
                }
            }
            
            goto screen_refreshed;
        }//32
        
        
        
        
        
        
        
        
        
        
        
        
        
        
        //assume <=256 colors using palette
        
        if (display_page->compatible_mode==10){//update SCREEN 10 palette
            i2=GetTicks()&512;
            if (i2) i2=1;
            for (i=0;i<=3;i++){
                display_page->pal[i]=pal_mode10[i2][display_page->pal[i+4]];//pal_mode10[0-1][0-8]
            }
        }
        
        i=display_page->width*display_page->height;
        i2=1<<display_page->bits_per_pixel;//unique colors
        
        //data changed?
        if (i!=pixeldatasize){
            pixeldata_storage.resize(i,0);
            pixeldata=pixeldata_storage.data();
            pixeldatasize=static_cast<int32>(pixeldata_storage.size());
            goto update_display;
        }
        
        if (force_display_update) goto update_display; //force update
        
        if (displayorder_screen==0){
            //a valid frame of the correct dimensions exists and we are not required to display software content
            goto no_new_frame;
        }
        
        if (memcmp(pixeldata,display_page->offset,i)) goto update_display;
        //palette changed?
        if (memcmp(paldata,display_page->pal,i2*4)) goto update_display;
        //force update because of mode change?
        if (!screen_last_valid) goto update_display;
        
        goto no_new_frame;//no need to update display
        
        update_display:
        
        
        //################################ Setup new frame ################################
        {
            static int32 new_size_bytes;
            new_size_bytes=x_monitor*y_monitor*4;
            if (new_size_bytes>display_frame[frame_i].bytes){
                display_frame[frame_i].bgra_storage.resize(static_cast<size_t>(new_size_bytes/4),0);
                display_frame[frame_i].bgra=display_frame[frame_i].bgra_storage.data();
                display_frame[frame_i].bytes=new_size_bytes;
            }
            display_frame[frame_i].w=x_monitor; display_frame[frame_i].h=y_monitor;
        }
        
        display_surface_offset=display_frame[frame_i].bgra;
        
        memcpy(pixeldata,display_page->offset,i);
        memcpy(paldata,display_page->pal,i2*4);
        
        if (BGRA_to_RGBA) swap_paldata_BGRA_with_RGBA();
        static uint8 *cp;
        static uint32 *lp2;
        static uint32 c;
        cp=pixeldata;
        lp2=display_surface_offset;
        x2=display_page->width;
        y2=display_page->height;
        for (y=0;y<y2;y++){
            for (x=0;x<x2;x++){
                *lp2++=paldata[*cp++];
            }//x
        }//y
        if (BGRA_to_RGBA) swap_paldata_BGRA_with_RGBA();
        
        goto screen_refreshed;
        
        
        
        
        
        
        
        
        screen_refreshed:
        
        force_display_update=0;
        
        screen_last_valid=1;
        
        //Set new display frame as ready
        //display_frame_end=frame_i;
        //if (!display_frame_begin) display_frame_begin=frame_i;
        
        display_frame[frame_i].state=DISPLAY_FRAME_STATE__READY; last_hardware_display_frame_order=display_frame[frame_i].order;

        
        no_new_frame:;
        display_page_invalid:;
        
        
        //cancel frame if not built
        if (display_frame[frame_i].state==DISPLAY_FRAME_STATE__BUILDING){
            last_hardware_display_frame_order=display_frame[frame_i].order;
            display_frame[frame_i].state=DISPLAY_FRAME_STATE__EMPTY;
        }
        present_frame();//R8: compose + hand to the window thread
    }//lock_display==0
    if (lock_display==1){lock_display=2; Sleep(0);}
    if (autodisplay==-1) autodisplay=0;
    display_called=1; 
    return;
}





/*
    int message_loop(
    SDL_Event event,
    SDL_Surface *screen,
    SDL_Surface *back,
    int *inputedWidth,
    Uint16 *inputedString,
    TTF_Font *font)
    {
    SDL_Color fg = {0x66, 0x66, 0xFF};
    SDL_Color bg = {0x00, 0x00, 0x00};
    SDL_Rect rect;
    SDL_Surface *surface;
    SDL_Event eventExpose;
    
    switch(event.type){
    case SDL_QUIT:
    return 1;
    case SDL_VIDEOEXPOSE:
    SDL_BlitSurface(back, NULL, screen, NULL);
    SDL_UpdateRect(screen, 0, 0, 0, 0);
    break;
    case SDL_KEYDOWN:
    
    
    
    
    
    if (event.key.keysym.sym == QBVK_F1) {
    InputMethod_Reset();
    }
    if (event.key.keysym.sym == QBVK_F2) {
    *inputedWidth = 0;
    inputedString[0] = 0x0000;
    InputMethod_Reset();
    }
    if (event.key.keysym.sym == QBVK_F3) {
    InputMethod_Validate();
    rect.x = 0;
    rect.y = 200;
    rect.w = 640;
    rect.h = 100;
    SDL_FillRect(back, &rect, 0x00000000);
    surface = TTF_RenderUTF8_Shaded(
    font, "Valid", fg, bg);
    SDL_BlitSurface(surface, NULL, back, &rect);
    SDL_FreeSurface(surface);
    eventExpose.type = SDL_VIDEOEXPOSE;
    SDL_PushEvent(&eventExpose);
    }
    if (event.key.keysym.sym == QBVK_F4) {
    InputMethod_Invalidate();
    rect.x = 0;
    rect.y = 200;
    rect.w = 640;
    rect.h = 100;
    SDL_FillRect(back, &rect, 0x00000000);
    surface = TTF_RenderUTF8_Shaded(
    font, "Invalid", fg, bg);
    SDL_BlitSurface(surface, NULL, back, &rect);
    SDL_FreeSurface(surface);
    eventExpose.type = SDL_VIDEOEXPOSE;
    SDL_PushEvent(&eventExpose);
    }
    break;
    default:
    break;
    }
    return 0;
    }
*/

void update_shift_state(){
    //R8: keybuf.rs computes 0:417h, 0:418h and 0:496h from the held keys
    uint8 b[3];
    qb_shift_state(b);
    cmem[0x417]=b[0];
    cmem[0x418]=b[1];
    cmem[0x496]=b[2];
}


int32 keyup_mask_last=-1;
uint32 keyup_mask[256];//NULL values indicate removed masks

void keyup(uint32 x){
    
    if (!x) x=QBK+QBK_CHR0;
    
    keyheld_remove(x);
    
    if (asciicode_reading!=2){//hide numpad presses related to ALT+1+2+3 type entries
        //identify and revert numpad specific key codes to non-numpad codes
        static uint32 x2;
        static int64 numpadkey;
        numpadkey=0;
        x2=x;
        //check multimapped NUMPAD keys
        if ((x>=(VK+QBVK_KP0))&&(x<=(VK+QBVK_KP_ENTER))){
            numpadkey=4294967296ll;
            if ((x>=(VK+QBVK_KP0))&&(x<=(VK+QBVK_KP9))){x2=x-(VK+QBVK_KP0)+48; goto onnumpad;}
            if (x==(VK+QBVK_KP_PERIOD)){x2=46; goto onnumpad;}
            if (x==(VK+QBVK_KP_DIVIDE)){x2=47; goto onnumpad;}
            if (x==(VK+QBVK_KP_MULTIPLY)){x2=42; goto onnumpad;}
            if (x==(VK+QBVK_KP_MINUS)){x2=45; goto onnumpad;}
            if (x==(VK+QBVK_KP_PLUS)){x2=43; goto onnumpad;}
            if (x==(VK+QBVK_KP_ENTER)){x2=13; goto onnumpad;}
        }
        if ((x>=(QBK+0))&&(x<=(QBK+10))){
            numpadkey=4294967296ll;
            x2=x-QBK;
            if (x2==0){x2=82<<8; goto onnumpad;}
            if (x2==1){x2=79<<8; goto onnumpad;}
            if (x2==2){x2=80<<8; goto onnumpad;}
            if (x2==3){x2=81<<8; goto onnumpad;}
            if (x2==4){x2=75<<8; goto onnumpad;}
            if (x2==5){x2=76<<8; goto onnumpad;}
            if (x2==6){x2=77<<8; goto onnumpad;}
            if (x2==7){x2=71<<8; goto onnumpad;}
            if (x2==8){x2=72<<8; goto onnumpad;}
            if (x2==9){x2=73<<8; goto onnumpad;}
            if (x2==10){x2=83<<8; goto onnumpad;}
        }
        onnumpad:;
        
        static int32 i;
        for (i=0;i<=keyup_mask_last;i++){
            if (x==keyup_mask[i]){
                keyup_mask[i]=0;
                goto key_handled;
            }
        }
        
        //add x2 to keyhit buffer
        static int32 sx;
        sx=x2; sx=-sx; x2=sx;//negate x2
        qb_keyhit_push(x2|numpadkey);
    }//asciicode_reading!=2
    
    
    static int32 shift,alt,ctrl,capslock,numlock;
    numlock=0; capslock=0;
    
    if (x<=255){
        if (scancode_lookup[x*10+2]) scancodeup(scancode_lookup[x*10+1]);
        goto key_handled;
    }//x<=255
    
    //NUMPAD?
    if ((x>=(VK+QBVK_KP0))&&(x<=(VK+QBVK_KP_ENTER))){
        if ((x>=(VK+QBVK_KP0))&&(x<=(VK+QBVK_KP_PERIOD))) numlock=1;
        x=(x-(VK+QBVK_KP0)+256)*256;
        goto numpadkey;
    }
    if ((x>=(QBK+0))&&(x<=(QBK+0+(QBVK_KP_PERIOD-QBVK_KP0)))){
        x=(x-(QBK+0)+256)*256;
        goto numpadkey;
    }
    
    if (x<=65535){
        static int32 r;
        numpadkey:
        r=(x>>8)+256;
        if (scancode_lookup[r*10+2]) scancodeup(scancode_lookup[r*10+1]);
        
        if (x==0x5200){//INSERT lock emulation
            update_shift_state();
        }
        
        goto key_handled;
    }//x<=65536
    
    if (x==(VK+QBVK_LSHIFT)){
        scancodeup(42);
        update_shift_state();
    }
    if (x==(VK+QBVK_RSHIFT)){
        scancodeup(54);
        update_shift_state();
    }
    if (x==(VK+QBVK_LALT)){
        scancodeup(56);
        update_shift_state();
    }
    if (x==(VK+QBVK_RALT)){
        scancodeup(56);
        update_shift_state();
    }
    if (x==(VK+QBVK_LCTRL)){
        scancodeup(29);
        update_shift_state();
    }
    if (x==(VK+QBVK_RCTRL)){
        scancodeup(29);
        update_shift_state();
    }
    if (x==(VK+QBVK_NUMLOCK)){
        scancodeup(69);
        update_shift_state();
    }
    if (x==(VK+QBVK_CAPSLOCK)){
        scancodeup(58);
        update_shift_state();
    }
    if (x==(VK+QBVK_SCROLLOCK)){
        scancodeup(70);
        update_shift_state();
    }
    
    key_handled:;
    
}

void keydown(uint32 x){
    
    if (!x) x=QBK+QBK_CHR0;
    
    static int32 glyph;
    glyph=keydown_glyph; keydown_glyph=0;
    
    //INSERT lock emulation
    static int32 insert_held;
    if (x==0x5200) insert_held=keyheld(0x5200);
    
    //SCROLL lock tracking
    static int32 scroll_lock_held;
    if (x==(VK+QBVK_SCROLLOCK)) scroll_lock_held=keyheld(VK+QBVK_SCROLLOCK);
    
    keyheld_add(x);
    
    //note: On early keyboards without a Pause key (before the introduction of 101-key keyboards) the Pause function was assigned to Ctrl+NumLock, and the Break function to Ctrl+ScrLock; these key-combinations still work with most programs, even on modern PCs with modern keyboards.
    //CTRL+BREAK handling
    if (
    (x==(VK+QBVK_BREAK))
    || ((x==(VK+QBVK_SCROLLOCK))&&(keyheld(VK+QBVK_LCTRL)||keyheld(VK+QBVK_RCTRL))) 
    || ((x==(VK+QBVK_F15))&&(keyheld(VK+QBVK_LCTRL)||keyheld(VK+QBVK_RCTRL))) 
    ){
        if (exit_blocked){exit_value|=2; goto key_handled;}
        close_program=1;
        goto key_handled;
    }
    
    #ifdef QBNex_WINDOWS
        //note: Alt+F4 is supposed to close the window, but glut windows don't seem to be affected;
        //this addresses the issue:
        if ( (x==(0x3E00)) && (keyheld(VK+QBVK_RALT)||keyheld(VK+QBVK_LALT)) ){
            if (exit_blocked){exit_value|=1; goto key_handled;}
            close_program=1;
            goto key_handled;
        }
    #endif    
    
    //note: On early keyboards without a Pause key (before the introduction of 101-key keyboards) the Pause function was assigned to Ctrl+NumLock, and the Break function to Ctrl+ScrLock; these key-combinations still work with most programs, even on modern PCs with modern keyboards.
    //PAUSE handling
    if ( (x==(VK+QBVK_PAUSE)) || ((x==(VK+QBVK_NUMLOCK))&&(keyheld(VK+QBVK_LCTRL)||keyheld(VK+QBVK_RCTRL))) ){
        suspend_program|=1;
        qbevent=1;
        goto key_handled;
        }else{
        if (suspend_program&1){
            suspend_program^=1;
            goto key_handled;
        }
    }
    
    //ALT+ENTER
    if (keyheld(VK+QBVK_RALT)||keyheld(VK+QBVK_LALT)){
        if (x==13){
            if (fullscreen_allowedmode>=0) { //fullscreen_allowedmode==-1 bypasses alt+enter allowing it to be user-trappable
                static int32 fs_mode,fs_smooth;
                fs_mode=full_screen_set;
                if (fs_mode==-1) fs_mode=full_screen;
                fs_smooth=fullscreen_smooth;
                
                int32 fs_combo;
                if (fs_mode==0) fs_combo=0;
                if ((fs_mode==1) && (fs_smooth==0)) fs_combo=1;
                if ((fs_mode==1) && (fs_smooth==1)) fs_combo=2;
                if ((fs_mode==2) && (fs_smooth==0)) fs_combo=3;
                if ((fs_mode==2) && (fs_smooth==1)) fs_combo=4;
                
                int32 fs_validmode=0;
                while (fs_validmode==0) {
                    fs_combo++;
                    if (fs_combo>4) fs_combo=0;
                    
                    switch (fs_combo) {
                        case 0:
                        fs_mode=0; fullscreen_smooth=0;
                        break;
                        case 1:
                        fs_mode=1; fullscreen_smooth=0;
                        break;
                        case 2:
                        fs_mode=1; fullscreen_smooth=1;
                        break;
                        case 3:
                        fs_mode=2; fullscreen_smooth=0;
                        break;
                        case 4:
                        fs_mode=2; fullscreen_smooth=1;
                        break;
                    }
                    
                    if (fs_combo==0) break; // 0 is always valid (= _OFF)
                    
                    fs_validmode=1;
                    //check _ALLOWFULLSCREEN's settings
                    if ((fullscreen_allowedmode>0)&&(fs_mode!=fullscreen_allowedmode)) fs_validmode=0;
                    if ((fullscreen_allowedsmooth==1)&&(fullscreen_smooth!=1)) fs_validmode=0;
                    if ((fullscreen_allowedsmooth==-1)&&(fullscreen_smooth!=0)) fs_validmode=0;
                }
                
                //apply
                if (full_screen!=fs_mode) full_screen_set=fs_mode;
                force_display_update=1;
                goto key_handled;
            }
        }
    }
    
    if (asciicode_reading!=2){//hide numpad presses related to ALT+1+2+3 type entries
        
        //identify and revert numpad specific key codes to non-numpad codes
        static uint32 x2;
        static int64 numpadkey;
        numpadkey=0;
        x2=x;
        //check multimapped NUMPAD keys
        if ((x>=(VK+QBVK_KP0))&&(x<=(VK+QBVK_KP_ENTER))){
            numpadkey=4294967296ll;
            if ((x>=(VK+QBVK_KP0))&&(x<=(VK+QBVK_KP9))){x2=x-(VK+QBVK_KP0)+48; goto onnumpad;}
            if (x==(VK+QBVK_KP_PERIOD)){x2=46; goto onnumpad;}
            if (x==(VK+QBVK_KP_DIVIDE)){x2=47; goto onnumpad;}
            if (x==(VK+QBVK_KP_MULTIPLY)){x2=42; goto onnumpad;}
            if (x==(VK+QBVK_KP_MINUS)){x2=45; goto onnumpad;}
            if (x==(VK+QBVK_KP_PLUS)){x2=43; goto onnumpad;}
            if (x==(VK+QBVK_KP_ENTER)){x2=13; goto onnumpad;}
        }
        if ((x>=(QBK+0))&&(x<=(QBK+10))){
            numpadkey=4294967296ll;
            x2=x-QBK;
            if (x2==0){x2=82<<8; goto onnumpad;}
            if (x2==1){x2=79<<8; goto onnumpad;}
            if (x2==2){x2=80<<8; goto onnumpad;}
            if (x2==3){x2=81<<8; goto onnumpad;}
            if (x2==4){x2=75<<8; goto onnumpad;}
            if (x2==5){x2=76<<8; goto onnumpad;}
            if (x2==6){x2=77<<8; goto onnumpad;}
            if (x2==7){x2=71<<8; goto onnumpad;}
            if (x2==8){x2=72<<8; goto onnumpad;}
            if (x2==9){x2=73<<8; goto onnumpad;}
            if (x2==10){x2=83<<8; goto onnumpad;}
        }
        onnumpad:;
        
        
        
        
        
        //ON KEY trapping
        {//new scope
            static int32 block_onkey=0;
            static int32 f,x3,scancode,extended,c,flags_mask;
            int32 i,i2;//must not be static!
            
            //establish scancode (if any)
            scancode=0;
            if (x<=255){scancode=scancode_lookup[x*10+1]; goto onkey_gotscancode;}
            //*check for 2 byte scancodes here
            x3=x;
            if ((x3>=(VK+QBVK_KP0))&&(x3<=(VK+QBVK_KP_ENTER))){
                x3=(x3-(VK+QBVK_KP0)+256)*256;
                goto onkey_numpadkey;
            }
            if ((x3>=(QBK+0))&&(x3<=(QBK+0+(QBVK_KP_PERIOD-QBVK_KP0)))){
                x3=(x3-(QBK+0)+256)*256;
                goto onkey_numpadkey;
            }
            if (x3<=65535){
                onkey_numpadkey:
                i=(x3>>8)+256;
                if (scancode_lookup[i*10+2]) scancode=scancode_lookup[i*10+1];
            }
            onkey_gotscancode:
            
            //check modifier keys
            if (x==(VK+QBVK_LSHIFT)){
                scancode=42;
                flags_mask=3;
            }
            if (x==(VK+QBVK_RSHIFT)){
                scancode=54;
                flags_mask=3;
            }
            if (x==(VK+QBVK_LALT)){
                scancode=56;
                flags_mask=8;
            }
            if (x==(VK+QBVK_RALT)){
                scancode=56;
                flags_mask=8;
            }
            if (x==(VK+QBVK_LCTRL)){
                scancode=29;
                flags_mask=4;
            }
            if (x==(VK+QBVK_RCTRL)){
                scancode=29;
                flags_mask=4;
            }
            if (x==(VK+QBVK_NUMLOCK)){
                scancode=69;
                flags_mask=32;
            }
            if (x==(VK+QBVK_CAPSLOCK)){
                scancode=58;
                flags_mask=64;
            }
            if (x==(VK+QBVK_SCROLLOCK)){
                scancode=70;
                //note: no mask required
            }
            
            //establish if key is an extended key
            extended=0;
            //arrow-pad (note: num-pad is ignored because x is a QBNex pure key value and only refers to the arrow-pad)
            if (x==0x4B00)extended=1;
            if (x==0x4800)extended=1;
            if (x==0x4D00)extended=1;
            if (x==0x5000)extended=1;
            //num-pad extended keys
            if (x==VK+QBVK_KP_DIVIDE)extended=1;
            if (x==VK+QBVK_KP_ENTER)extended=1;
            //ins/del/hom/end/pgu/pgd pad
            if (x==0x5200)extended=1;
            if (x==0x4700)extended=1;
            if (x==0x4900)extended=1;
            if (x==0x5300)extended=1;
            if (x==0x4F00)extended=1;
            if (x==0x5100)extended=1;
            //right alt/right control
            if (x==VK+QBVK_RCTRL)extended=1;
            if (x==VK+QBVK_RALT)extended=1;
            
            if (!block_onkey){
                
                //priority #1: user defined keys
                if (scancode){
                    for (i=0;i<=31;i++){
                        if (onkey[i].key_scancode==scancode){
                            if (onkey[i].active){
                                if (onkey[i].id){
                                    //check keyboard flags
                                    f=onkey[i].key_flags;
                                    //0 No keyboard flag, 1-3 Either Shift key, 4 Ctrl key, 8 Alt key,32 NumLock key,64 Caps Lock key, 128 Extended keys on a 101-key keyboard
                                    //To specify multiple shift states, add the values together. For example, a value of 12 specifies that the user-defined key is used in combination with the Ctrl and Alt keys.
                                    if ((flags_mask&3)==0){
                                        if (f&3){
                                            if (keyheld(VK+QBVK_LSHIFT)==0&&keyheld(VK+QBVK_RSHIFT)==0) goto wrong_flags;
                                            }else{
                                            if (keyheld(VK+QBVK_LSHIFT)||keyheld(VK+QBVK_RSHIFT)) goto wrong_flags;
                                        }
                                    }
                                    if ((flags_mask&4)==0){
                                        if (f&4){
                                            if (keyheld(VK+QBVK_LCTRL)==0&&keyheld(VK+QBVK_RCTRL)==0) goto wrong_flags;
                                            }else{
                                            if (keyheld(VK+QBVK_LCTRL)||keyheld(VK+QBVK_RCTRL)) goto wrong_flags;
                                        }
                                    }
                                    if ((flags_mask&8)==0){
                                        if (f&8){
                                            if (keyheld(VK+QBVK_LALT)==0&&keyheld(VK+QBVK_RALT)==0) goto wrong_flags;
                                            }else{
                                            if (keyheld(VK+QBVK_LALT)||keyheld(VK+QBVK_RALT)) goto wrong_flags;
                                        }
                                    }
                                    if ((flags_mask&32)==0){
                                        if (f&32){
                                            if (keyheld(VK+QBVK_NUMLOCK)==0) goto wrong_flags;
                                            //*revise
                                        }
                                    }
                                    if ((flags_mask&64)==0){
                                        if (f&64){
                                            if (keyheld(VK+QBVK_CAPSLOCK)==0) goto wrong_flags;
                                            //*revise
                                        }
                                    }
                                    if ((flags_mask&128)==0){
                                        if (((f&128)/128)!=extended) goto wrong_flags;
                                    }
                                    if (onkey[i].active==1){//(1)ON
                                        onkey[i].state++;
                                        }else{//(2)STOP
                                        onkey[i].state=1;
                                    }
                                    
                                    qbevent=1;
                                    
                                    //mask trigger key
                                    for (i=0;i<=keyup_mask_last;i++){
                                        if (!keyup_mask[i]){
                                            keyup_mask[i]=x;  
                                            break;
                                        }
                                    }
                                    if (i==keyup_mask_last+1){
                                        if (keyup_mask_last<255){
                                            keyup_mask[i]=x;
                                            keyup_mask_last++;
                                        }
                                    }
                                    
                                    goto key_handled;
                                    
                                }//id
                            }//active
                        }//scancode==
                        wrong_flags:;
                    }//i
                }//scancode
                
                //priority #2: fixed index F1-F12, arrows
                for (i=0;i<=31;i++){
                    if (onkey[i].active){
                        if (onkey[i].id){
                            if ((x2==onkey[i].keycode)||x==onkey[i].keycode_alternate){
                                if (onkey[i].active==1){//(1)ON
                                    onkey[i].state++;
                                    }else{//(2)STOP
                                    onkey[i].state=1;
                                }
                                qbevent=1;
                                
                                //mask trigger key
                                for (i=0;i<=keyup_mask_last;i++){
                                    if (!keyup_mask[i]){
                                        keyup_mask[i]=x;  
                                        break;
                                    }
                                }
                                if (i==keyup_mask_last+1){
                                    if (keyup_mask_last<255){
                                        keyup_mask[i]=x;
                                        keyup_mask_last++;
                                    }
                                }
                                
                                goto key_handled;
                            }//keycode
                        }//id
                    }//active
                }//i
                
            }//block_onkey
            
            //priority #3: string insertion
            for (i=0;i<=31;i++){
                if (onkey[i].text){
                    if (onkey[i].text->len){
                        if ((x2==onkey[i].keycode)||x==onkey[i].keycode_alternate){
                            
                            //mask trigger key
                            {//scope
                                static int32 i;
                                for (i=0;i<=keyup_mask_last;i++){
                                    if (!keyup_mask[i]){
                                        keyup_mask[i]=x;  
                                        break;
                                    }
                                }
                                if (i==keyup_mask_last+1){
                                    if (keyup_mask_last<255){
                                        keyup_mask[i]=x;
                                        keyup_mask_last++;
                                    }
                                }
                            }//descope
                            
                            for (i2=0;i2<onkey[i].text->len;i2++){
                                block_onkey=1;
                                keydown_ascii(onkey[i].text->chr[i2]);
                                keyup_ascii(onkey[i].text->chr[i2]);
                                block_onkey=0;
                            }//i2
                            goto key_handled;
                        }//keycode
                    }}//text
            }//i
            
        }//descope
        
        
        /*
            //keyhit cyclic buffer
            int64 keyhit[8192];
            //    keyhit specific internal flags: (stored in high 32-bits)
            //    &4294967296->numpad was used
            int32 keyhit_nextfree=0;
            int32 keyhit_next=0;
            //note: if full, the oldest message is discarded to make way for the new message
        */
        //add x2 to keyhit buffer
        qb_keyhit_push(x2|numpadkey);
    }//asciicode_reading!=2
    
    
    static int32 shift,alt,ctrl,capslock,numlock;
    numlock=0; capslock=0;
    
    if (x==QBK+QBK_CHR0) x=0;
    
    if (x<=255){
        static int32 b1,b2,z,o;
        b1=x;
        if (b2=scancode_lookup[x*10+1]){//table entry exists
            scancodedown(b2);
            
            
            
            
            
            
            
            //check for relevent table modifiers
            shift=0; if (keyheld(VK+QBVK_LSHIFT)||keyheld(VK+QBVK_RSHIFT)) shift=1;
            ctrl=0; if (keyheld(VK+QBVK_LCTRL)||keyheld(VK+QBVK_RCTRL)) ctrl=1;
            alt=0; if (keyheld(VK+QBVK_LALT)||keyheld(VK+QBVK_RALT)) alt=1;
            o=0;
            if (shift) o=1;
            if (ctrl) o=2;
            if (alt) o=3;
            if (glyph){
                if ((keyheld(VK+QBVK_LALT)==0)&&(keyheld(VK+QBVK_RCTRL)==0)&&keyheld(VK+QBVK_LCTRL)&&keyheld(VK+QBVK_RALT)) o=0;//assume alt-gr combo-key
            }
            z=scancode_lookup[x*10+2+o];
            if (!z) goto key_handled;//not possible
            if (z&0xFF00){
                b1=0;
                b2=z>>8;
                }else{
                b1=z;
            }
        }//b2
        static int32 i,i2,i3;
        i=cmem[0x41a];
        i2=cmem[0x41c];
        i3=i2+2;
        if (i3==62) i3=30;
        if (i!=i3){//fits in buffer
            cmem[0x400+i2]=b1;
            cmem[0x400+i2+1]=b2;//(scancode)
            cmem[0x41c]=i3;//fix tail location
        }
        goto key_handled;
    }//x<=255
    
    //NUMPAD?
    if ((x>=(VK+QBVK_KP0))&&(x<=(VK+QBVK_KP_ENTER))){
        if ((x>=(VK+QBVK_KP0))&&(x<=(VK+QBVK_KP_PERIOD))) numlock=1;
        x=(x-(VK+QBVK_KP0)+256)*256;
        goto numpadkey;
    }
    if ((x>=(QBK+0))&&(x<=(QBK+0+(QBVK_KP_PERIOD-QBVK_KP0)))){
        x=(x-(QBK+0)+256)*256;
        goto numpadkey;
    }
    
    if (x<=65535){
        static int32 b1,b2,z,o,r;
        numpadkey:
        b1=0; b2=x>>8;
        r=(x>>8)+256;
        if (scancode_lookup[r*10+2]){
            scancodedown(scancode_lookup[r*10+1]);
            //check relevent modifiers
            shift=0; if (keyheld(VK+QBVK_LSHIFT)||keyheld(VK+QBVK_RSHIFT)) shift=1;
            ctrl=0; if (keyheld(VK+QBVK_LCTRL)||keyheld(VK+QBVK_RCTRL)) ctrl=1;
            alt=0; if (keyheld(VK+QBVK_LALT)||keyheld(VK+QBVK_RALT)) alt=1;
            
            if (x==0x5200){//INSERT lock emulation
                if (insert_held==0){//nullify effects of key repeats
                    if ((alt==0)&&(shift==0)&&(ctrl==0)){
                        //toggle insert mode
                        if (keyheld(QBK+QBK_INSERT_MODE)){
                            keyheld_remove(QBK+QBK_INSERT_MODE);
                            }else{
                            keyheld_add(QBK+QBK_INSERT_MODE);
                        }
                        update_shift_state();
                    }
                }
            }
            
            o=0;
            if (shift) o=1;
            if (numlock) o=4;
            if (numlock&&shift) o=7;
            if (ctrl) o=2;
            if (alt) o=3;
            z=scancode_lookup[r*10+2+o];
            if (!z) goto key_handled;//invalid combination
            if (z&0xFF00){
                b1=0;
                b2=z>>8;
                }else{
                b1=z;
                b2=scancode_lookup[r*10+1];
            }
        }//z
        static int32 i,i2,i3;
        i=cmem[0x41a];
        i2=cmem[0x41c];
        i3=i2+2;
        if (i3==62) i3=30;
        if (i!=i3){//fits in buffer
            cmem[0x400+i2]=b1;
            cmem[0x400+i2+1]=b2;//(scancode)
            cmem[0x41c]=i3;//fix tail location
        }
        goto key_handled;
    }//x<=65536
    
    if (x==(VK+QBVK_LSHIFT)){
        scancodedown(42);
        update_shift_state();
    }
    if (x==(VK+QBVK_RSHIFT)){
        scancodedown(54);
        update_shift_state();
    }
    if (x==(VK+QBVK_LALT)){
        scancodedown(56);
        update_shift_state();
    }
    if (x==(VK+QBVK_RALT)){
        scancodedown(56);
        update_shift_state();
    }
    if (x==(VK+QBVK_LCTRL)){
        scancodedown(29);
        update_shift_state();
    }
    if (x==(VK+QBVK_RCTRL)){
        scancodedown(29);
        update_shift_state();
    }
    if (x==(VK+QBVK_NUMLOCK)){
        scancodedown(69);
        update_shift_state();
    }
    if (x==(VK+QBVK_CAPSLOCK)){
        scancodedown(58);
        update_shift_state();
    }
    if (x==(VK+QBVK_SCROLLOCK)){
        scancodedown(70);
        
        if (scroll_lock_held==0){//nullify effects of key repeats
            ctrl=0; if (keyheld(VK+QBVK_LCTRL)||keyheld(VK+QBVK_RCTRL)) ctrl=1;
            if (ctrl==0){
                //toggle insert mode
                if (keyheld(QBK+QBK_SCROLL_LOCK_MODE)){
                    keyheld_remove(QBK+QBK_SCROLL_LOCK_MODE);
                    }else{
                    keyheld_add(QBK+QBK_SCROLL_LOCK_MODE);
                }
            }
        }
        
        update_shift_state();
    }
    
    key_handled:
    sleep_break=1;
    
}

void scancodedown(uint8 scancode){
    if (port60h_events){
        if (port60h_event[port60h_events-1]==scancode) return;//avoid duplicate entries in buffer (eg. from key-repeats)
    }
    if (port60h_events==256){memmove(port60h_event,port60h_event+1,255); port60h_events=255;}
    port60h_event[port60h_events]=scancode;
    port60h_events++;
}

void scancodeup(uint8 scancode){
    if (port60h_events){
        if (port60h_event[port60h_events-1]==(scancode+128)) return;//avoid duplicate entries in buffer
    }
    if (port60h_events==256){memmove(port60h_event,port60h_event+1,255); port60h_events=255;}
    port60h_event[port60h_events]=scancode+128;
    port60h_events++;
}





int32 func__capslock(){
    #ifdef QBNex_WINDOWS
    return -GetKeyState(VK_CAPITAL);
    #endif
    return 0;
}

int32 func__scrolllock(){
    #ifdef QBNex_WINDOWS
    return -GetKeyState(VK_SCROLL);
    #endif
    return 0;
}

int32 func__numlock(){
    #ifdef QBNex_WINDOWS
    return -GetKeyState(VK_NUMLOCK);
    #endif
    return 0;
}

void toggle_lock_key(int32 key_code){
    #ifdef QBNex_WINDOWS
    keybd_event (key_code, 0x45, 1, 0);
    keybd_event (key_code, 0x45, 3, 0);
    #endif
}

void sub__capslock(int32 options){
    #ifdef QBNex_WINDOWS
    //VK_CAPITAL
    int32 currentState = func__capslock();
    switch(options){
        case 1: //ON
            if (currentState==-1) return;
            break;
        case 2: //OFF
            if (currentState==0) return;
            break;
    }
    // _TOGGLE:
    toggle_lock_key(VK_CAPITAL);
    #endif
}

void sub__scrolllock(int32 options){
    #ifdef QBNex_WINDOWS
    //VK_SCROLL
    int32 currentState = func__scrolllock();
    switch(options){
        case 1: //ON
            if (currentState==-1) return;
            break;
        case 2: //OFF
            if (currentState==0) return;
            break;
    }
    // _TOGGLE:
    toggle_lock_key(VK_SCROLL);
    #endif
}

void sub__numlock(int32 options){
    #ifdef QBNex_WINDOWS
    //VK_NUMLOCK
    int32 currentState = func__numlock();
    switch(options){
        case 1: //ON
            if (currentState==-1) return;
            break;
        case 2: //OFF
            if (currentState==0) return;
            break;
    }
    // _TOGGLE:
    toggle_lock_key(VK_NUMLOCK);
    #endif
}

void sub__consolefont(qbs* FontName, int FontSize){
    #ifdef QBNex_WINDOWS
	#if WINVER >= 0x0600 //this block is not compatible with XP
    SECURITY_ATTRIBUTES SecAttribs = {sizeof(SECURITY_ATTRIBUTES), 0, 1};
    HANDLE cl_conout = CreateFileA("CONOUT$", GENERIC_READ | GENERIC_WRITE, FILE_SHARE_READ | FILE_SHARE_WRITE, & SecAttribs, OPEN_EXISTING, 0, 0);
    static int OneTimePause;
    if (!OneTimePause){ // a slight delay so the console can be properly created and registered with Windows, before we try and change fonts with it.       
        Sleep(500); 
        OneTimePause=1; //after the first pause, the console should be created, so we don't need any more delays in the future.
    }
    CONSOLE_FONT_INFOEX info = {0};
    info.cbSize       = sizeof(info);
    info.dwFontSize.Y = FontSize; // leave X as zero
    info.FontWeight   = FW_NORMAL;
    if (FontName->len>0){ //if we don't pass a font name, don't change the existing one.
        const size_t cSize = FontName->len;
        wchar_t* wc = new wchar_t[32];
        mbstowcs (wc, (char *)FontName->chr, cSize);
        wcscpy(info.FaceName, wc);
        delete[] wc;
    }

    SetCurrentConsoleFontEx(cl_conout, NULL, &info);
    #endif
	#endif
}


void sub__console_cursor(int32 visible, int32 cursorsize, int32 passed){
    #ifdef QBNex_WINDOWS
    HANDLE consoleHandle = GetStdHandle(STD_OUTPUT_HANDLE);
    CONSOLE_CURSOR_INFO info;
    
    GetConsoleCursorInfo(consoleHandle, &info); //get the original info, so we reuse it, unless the user called for a change.

    if (visible==1)info.bVisible = TRUE; //cursor is set to show
    if (visible==2)info.bVisible = FALSE; //set to hide
    if (passed&&cursorsize>=0&&cursorsize<=100)info.dwSize = cursorsize;  //the user passed the cursor size, of a suitable size
        
    SetConsoleCursorInfo(consoleHandle, &info);
    #endif
}

int32 func__getconsoleinput(){
    #ifdef QBNex_WINDOWS
    HANDLE hStdin = GetStdHandle (STD_INPUT_HANDLE);
    INPUT_RECORD irInputRecord;
    DWORD dwEventsRead, fdwMode, dwMode;
    CONSOLE_SCREEN_BUFFER_INFO cl_bufinfo;
 
    GetConsoleMode(hStdin, (LPDWORD)&dwMode);
    fdwMode = ENABLE_EXTENDED_FLAGS;
    SetConsoleMode(hStdin, fdwMode);
    fdwMode = dwMode | ENABLE_WINDOW_INPUT | ENABLE_MOUSE_INPUT;
    SetConsoleMode(hStdin, fdwMode);

    ReadConsoleInputA (hStdin, &irInputRecord, 1, &dwEventsRead);
    switch(irInputRecord.EventType){
        case KEY_EVENT: //keyboard input
            consolekey = irInputRecord.Event.KeyEvent.wVirtualScanCode;
            if (!irInputRecord.Event.KeyEvent.bKeyDown) consolekey = -consolekey; //positive/negative return of scan codes.
            return 1;
        case MOUSE_EVENT: //mouse input
            consolemousex = irInputRecord.Event.MouseEvent.dwMousePosition.X + 1;
            consolemousey = irInputRecord.Event.MouseEvent.dwMousePosition.Y - cl_bufinfo.srWindow.Top + 1; 
            consolebutton = irInputRecord.Event.MouseEvent.dwButtonState; //button state for all buttons
            //SetConsoleMode(hStdin, dwMode);
            return 2;
    }
    #endif
    return 0; //in case it's some other odd input
}

int32 func__cinp (int32 toggle, int32 passed){
    #ifdef QBNex_WINDOWS
        int32 temp = consolekey;
        consolekey = 0; //reset the console key, now that we've read it
        if (passed==0)toggle=1; //default return of positive/negative scan code values
        if (toggle) {
            return temp;
        }else{
            if (temp>=0)return temp;
            return -temp + 128;
        }
    #else
        return 0;
    #endif
}
