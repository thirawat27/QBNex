// Verbatim copy of legacy func_timer from internal/c/libqb.cpp (commit 545b859, lines 14030-14055).
// Test-only reference oracle for diff_timer.cpp; production uses crates/qb-runtime/src/timer.rs.
double func_timer(double accuracy,int32 passed){
    if (new_error) return 0;
    static uint32 x;
    static double d;
    static float f;
    x=GetTicks();
    x-=clock_firsttimervalue;
    x+=qbnex_firsttimervalue;
    //make timer value loop after midnight
    //note: there are 86400000 milliseconds in 24hrs(1 day)
    x%=86400000;
    d=x;//convert to double
    d/=1000.0;//convert from ms to sec
    //reduce accuracy
    if (!passed){
        accuracy=18.2;
        }else{
        if (accuracy<=0.0){error(5); return 0;}
        accuracy=1.0/accuracy;
    }
    d*=accuracy;
    d=qbr(d);
    d/=accuracy;
    if (!passed){f=d; d=f;}
    return d;
}
