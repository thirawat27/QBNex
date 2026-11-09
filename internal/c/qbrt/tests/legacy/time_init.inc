// Verbatim copy of legacy libqb.cpp (commit 545b859) lines 28654-28685: start-of-day setup in main(). Test-only.
    //call both timing routines as close as possible to each other to maximize accuracy
    //wait for second "hand" to "tick over"/move
    time(&qbnex_tm_val_old);
    //note: time() returns the time as seconds elapsed since midnight, January 1, 1970, or -1 in the case of an error. 
    if (qbnex_tm_val_old!=-1){
        do{
            time(&qbnex_tm_val);
        }while(qbnex_tm_val==qbnex_tm_val_old);
        }else{
        qbnex_tm_val=0;//time unknown! (set to midnight, January 1, 1970)
    }
    clock_firsttimervalue=GetTicks();
    //calculate localtime as milliseconds past midnight
    qbnex_tm=localtime(&qbnex_tm_val);
    /* re: localtime()
        Return a pointer to the structure result, or NULL if the date passed to the function is:
        Before midnight, January 1, 1970.
        After 03:14:07, January 19, 2038, UTC (using _time32 and time32_t).
        After 23:59:59, December 31, 3000, UTC (using _time64 and __time64_t).
    */
    if (qbnex_tm){
        qbnex_firsttimervalue=qbnex_tm->tm_hour*3600+qbnex_tm->tm_min*60+qbnex_tm->tm_sec;
        qbnex_firsttimervalue*=1000;
        }else{
        qbnex_firsttimervalue=0;//time unknown! (set to midnight)
    }
    /* Used as follows for calculating TIMER value:
        x=GetTicks();
        x-=clock_firsttimervalue;
        x+=qbnex_firsttimervalue;
    */
    
