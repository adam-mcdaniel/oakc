#[std]

const JAN = 0;
const FEB = JAN + 1;
const MAR = FEB + 1;
const APR = MAR + 1;
const MAY = APR + 1;
const JUN = MAY + 1;
const JUL = JUN + 1;
const AUG = JUL + 1;
const SEP = AUG + 1;
const OCT = SEP + 1;
const NOV = OCT + 1;
const DEC = NOV + 1;

fn month_name(m: num) -> &char {
    return m == JAN? "january"
        : m == FEB? "february"
        : m == MAR? "march"
        : m == APR? "april"
        : m == MAY? "may"
        : m == JUN? "june"
        : m == JUL? "july"
        : m == AUG? "august"
        : m == SEP? "september"
        : m == OCT? "october"
        : m == NOV? "november"
        : m == DEC? "december"
        : "unknown month"
}


fn is_leapyear(year: num) -> bool {
    if year % 4 == 0 && year % 100 != 0 {
        return true
    } else if year % 100 == 0 && year % 400 == 0 {
        return true
    } else {
        return false
    }
}


fn days_in_month(month: num, year: num) -> num {
    if month == FEB {
        return 28 + (is_leapyear(year) as num)
    } else {
        return 31 - ((month % 7) % 2)
    }
}

fn test_year(year: num) {
    putnum(year); putstr(" is leapyear? => ");
    putboolln(is_leapyear(year));
}

fn main() {
    for i in 1995..2005 {
        test_year(i);
    }
    test_year(2100);
    test_year(2400);

    let year = 2000;
    for i in JAN..DEC+1 {
        putstr(month_name(i));
        putstr(" has ");
        putnum(days_in_month(i, year));
        putstr(" in the year ");
        putnumln(year);
    }
}