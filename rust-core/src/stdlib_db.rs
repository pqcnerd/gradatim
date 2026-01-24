#[derive(Debug)]
pub struct ParamSpec {
    pub typ: &'static str,
    pub name: &'static str,
    pub desc: &'static str,
}

#[derive(Debug)]
pub struct FunctionSpec {
    pub name: &'static str,
    pub header: &'static str,
    pub return_type: &'static str,
    pub params: &'static [ParamSpec],
    pub patterns: &'static [&'static str],
}

#[derive(Debug)]
pub struct FunctionDatabase {
    pub functions: &'static [FunctionSpec],
}

impl FunctionDatabase {
    pub fn core() -> &'static FunctionDatabase {
        &CORE_DB
    }

    pub fn by_header<'a>(&'a self, header: &'a str) -> impl Iterator<Item = &'a FunctionSpec> + 'a {
        self.functions
            .iter()
            .filter(move |f| f.header.eq_ignore_ascii_case(header))
    }

    pub fn all(&self) -> impl Iterator<Item = &FunctionSpec> {
        self.functions.iter()
    }
}

// Core headers: stdio, stdlib, string, math
static CORE_FUNCS: &[FunctionSpec] = &[
    // stdio.h
    FunctionSpec {
        name: "printf",
        header: "stdio.h",
        return_type: "int",
        params: &[
            ParamSpec { typ: "const char *", name: "format", desc: "format string" },
        ],
        patterns: &["printf", "print formatted", "print format", "print"],
    },
    FunctionSpec {
        name: "fprintf",
        header: "stdio.h",
        return_type: "int",
        params: &[
            ParamSpec { typ: "FILE *", name: "stream", desc: "file stream" },
            ParamSpec { typ: "const char *", name: "format", desc: "format string" },
        ],
        patterns: &["fprintf", "print to file", "write formatted to file"],
    },
    FunctionSpec {
        name: "fopen",
        header: "stdio.h",
        return_type: "FILE *",
        params: &[
            ParamSpec { typ: "const char *", name: "path", desc: "path" },
            ParamSpec { typ: "const char *", name: "mode", desc: "mode" },
        ],
        patterns: &["open file", "fopen"],
    },
    FunctionSpec {
        name: "fclose",
        header: "stdio.h",
        return_type: "int",
        params: &[
            ParamSpec { typ: "FILE *", name: "stream", desc: "file stream" },
        ],
        patterns: &["close file", "fclose"],
    },
    FunctionSpec {
        name: "fread",
        header: "stdio.h",
        return_type: "size_t",
        params: &[
            ParamSpec { typ: "void *", name: "ptr", desc: "buffer" },
            ParamSpec { typ: "size_t", name: "size", desc: "size" },
            ParamSpec { typ: "size_t", name: "nmemb", desc: "count" },
            ParamSpec { typ: "FILE *", name: "stream", desc: "file" },
        ],
        patterns: &["fread", "read file buffer"],
    },
    FunctionSpec {
        name: "fwrite",
        header: "stdio.h",
        return_type: "size_t",
        params: &[
            ParamSpec { typ: "const void *", name: "ptr", desc: "buffer" },
            ParamSpec { typ: "size_t", name: "size", desc: "size" },
            ParamSpec { typ: "size_t", name: "nmemb", desc: "count" },
            ParamSpec { typ: "FILE *", name: "stream", desc: "file" },
        ],
        patterns: &["fwrite", "write file buffer"],
    },
    FunctionSpec {
        name: "fgets",
        header: "stdio.h",
        return_type: "char *",
        params: &[
            ParamSpec { typ: "char *", name: "s", desc: "buffer" },
            ParamSpec { typ: "int", name: "size", desc: "size" },
            ParamSpec { typ: "FILE *", name: "stream", desc: "file" },
        ],
        patterns: &["fgets", "read line", "read line from file"],
    },
    FunctionSpec {
        name: "fputs",
        header: "stdio.h",
        return_type: "int",
        params: &[
            ParamSpec { typ: "const char *", name: "s", desc: "string" },
            ParamSpec { typ: "FILE *", name: "stream", desc: "file" },
        ],
        patterns: &["fputs", "write string", "write string to file"],
    },
    FunctionSpec {
        name: "scanf",
        header: "stdio.h",
        return_type: "int",
        params: &[ParamSpec { typ: "const char *", name: "format", desc: "format string" }],
        patterns: &["scanf", "scan input", "read formatted"],
    },
    FunctionSpec {
        name: "fscanf",
        header: "stdio.h",
        return_type: "int",
        params: &[
            ParamSpec { typ: "FILE *", name: "stream", desc: "file stream" },
            ParamSpec { typ: "const char *", name: "format", desc: "format string" },
        ],
        patterns: &["fscanf", "read formatted from file"],
    },
    FunctionSpec {
        name: "sscanf",
        header: "stdio.h",
        return_type: "int",
        params: &[
            ParamSpec { typ: "const char *", name: "str", desc: "input string" },
            ParamSpec { typ: "const char *", name: "format", desc: "format string" },
        ],
        patterns: &["sscanf", "scan from string", "parse formatted string"],
    },
    FunctionSpec {
        name: "getchar",
        header: "stdio.h",
        return_type: "int",
        params: &[],
        patterns: &["getchar", "read char", "read character"],
    },
    FunctionSpec {
        name: "putchar",
        header: "stdio.h",
        return_type: "int",
        params: &[ParamSpec { typ: "int", name: "c", desc: "char" }],
        patterns: &["putchar", "write char", "print character"],
    },
    FunctionSpec {
        name: "fseek",
        header: "stdio.h",
        return_type: "int",
        params: &[
            ParamSpec { typ: "FILE *", name: "stream", desc: "file stream" },
            ParamSpec { typ: "long", name: "offset", desc: "offset" },
            ParamSpec { typ: "int", name: "whence", desc: "origin" },
        ],
        patterns: &["fseek", "seek file", "move file pointer"],
    },
    FunctionSpec {
        name: "ftell",
        header: "stdio.h",
        return_type: "long",
        params: &[ParamSpec { typ: "FILE *", name: "stream", desc: "file stream" }],
        patterns: &["ftell", "file position", "tell file position"],
    },
    FunctionSpec {
        name: "rewind",
        header: "stdio.h",
        return_type: "void",
        params: &[ParamSpec { typ: "FILE *", name: "stream", desc: "file stream" }],
        patterns: &["rewind", "rewind file", "reset file position"],
    },
    FunctionSpec {
        name: "fflush",
        header: "stdio.h",
        return_type: "int",
        params: &[ParamSpec { typ: "FILE *", name: "stream", desc: "file stream" }],
        patterns: &["fflush", "flush file", "flush output"],
    },
    FunctionSpec {
        name: "feof",
        header: "stdio.h",
        return_type: "int",
        params: &[ParamSpec { typ: "FILE *", name: "stream", desc: "file stream" }],
        patterns: &["feof", "end of file", "eof check"],
    },
    FunctionSpec {
        name: "ferror",
        header: "stdio.h",
        return_type: "int",
        params: &[ParamSpec { typ: "FILE *", name: "stream", desc: "file stream" }],
        patterns: &["ferror", "file error", "error check"],
    },

    // stdlib.h
    FunctionSpec {
        name: "malloc",
        header: "stdlib.h",
        return_type: "void *",
        params: &[ParamSpec { typ: "size_t", name: "size", desc: "size" }],
        patterns: &["malloc", "allocate", "allocate bytes"],
    },
    FunctionSpec {
        name: "calloc",
        header: "stdlib.h",
        return_type: "void *",
        params: &[
            ParamSpec { typ: "size_t", name: "nmemb", desc: "count" },
            ParamSpec { typ: "size_t", name: "size", desc: "size" },
        ],
        patterns: &["calloc", "allocate and zero"],
    },
    FunctionSpec {
        name: "realloc",
        header: "stdlib.h",
        return_type: "void *",
        params: &[
            ParamSpec { typ: "void *", name: "ptr", desc: "pointer" },
            ParamSpec { typ: "size_t", name: "size", desc: "size" },
        ],
        patterns: &["realloc", "reallocate"],
    },
    FunctionSpec {
        name: "free",
        header: "stdlib.h",
        return_type: "void",
        params: &[ParamSpec { typ: "void *", name: "ptr", desc: "pointer" }],
        patterns: &["free", "deallocate", "release memory"],
    },
    FunctionSpec {
        name: "exit",
        header: "stdlib.h",
        return_type: "void",
        params: &[ParamSpec { typ: "int", name: "status", desc: "status" }],
        patterns: &["exit", "exit with", "quit with"],
    },
    FunctionSpec {
        name: "rand",
        header: "stdlib.h",
        return_type: "int",
        params: &[],
        patterns: &["rand", "random", "random number"],
    },
    FunctionSpec {
        name: "srand",
        header: "stdlib.h",
        return_type: "void",
        params: &[ParamSpec { typ: "unsigned int", name: "seed", desc: "seed" }],
        patterns: &["srand", "seed random"],
    },
    FunctionSpec {
        name: "atoi",
        header: "stdlib.h",
        return_type: "int",
        params: &[ParamSpec { typ: "const char *", name: "nptr", desc: "string" }],
        patterns: &["atoi", "convert to int"],
    },
    FunctionSpec {
        name: "atof",
        header: "stdlib.h",
        return_type: "double",
        params: &[ParamSpec { typ: "const char *", name: "nptr", desc: "string" }],
        patterns: &["atof", "convert to float"],
    },
    FunctionSpec {
        name: "strtol",
        header: "stdlib.h",
        return_type: "long",
        params: &[
            ParamSpec { typ: "const char *", name: "nptr", desc: "string" },
            ParamSpec { typ: "char **", name: "endptr", desc: "end pointer" },
            ParamSpec { typ: "int", name: "base", desc: "base" },
        ],
        patterns: &["strtol", "convert to long"],
    },
    FunctionSpec {
        name: "strtod",
        header: "stdlib.h",
        return_type: "double",
        params: &[
            ParamSpec { typ: "const char *", name: "nptr", desc: "string" },
            ParamSpec { typ: "char **", name: "endptr", desc: "end pointer" },
        ],
        patterns: &["strtod", "convert to double"],
    },
    FunctionSpec {
        name: "qsort",
        header: "stdlib.h",
        return_type: "void",
        params: &[
            ParamSpec { typ: "void *", name: "base", desc: "array" },
            ParamSpec { typ: "size_t", name: "nmemb", desc: "count" },
            ParamSpec { typ: "size_t", name: "size", desc: "element size" },
            ParamSpec { typ: "int (*)(const void *, const void *)", name: "compar", desc: "compare" },
        ],
        patterns: &["qsort", "sort array", "quick sort"],
    },
    FunctionSpec {
        name: "bsearch",
        header: "stdlib.h",
        return_type: "void *",
        params: &[
            ParamSpec { typ: "const void *", name: "key", desc: "key" },
            ParamSpec { typ: "const void *", name: "base", desc: "array" },
            ParamSpec { typ: "size_t", name: "nmemb", desc: "count" },
            ParamSpec { typ: "size_t", name: "size", desc: "element size" },
            ParamSpec { typ: "int (*)(const void *, const void *)", name: "compar", desc: "compare" },
        ],
        patterns: &["bsearch", "binary search"],
    },
    FunctionSpec {
        name: "system",
        header: "stdlib.h",
        return_type: "int",
        params: &[ParamSpec { typ: "const char *", name: "command", desc: "command" }],
        patterns: &["system", "run command", "execute command"],
    },
    FunctionSpec {
        name: "getenv",
        header: "stdlib.h",
        return_type: "char *",
        params: &[ParamSpec { typ: "const char *", name: "name", desc: "env var" }],
        patterns: &["getenv", "get environment", "read env"],
    },
    FunctionSpec {
        name: "abort",
        header: "stdlib.h",
        return_type: "void",
        params: &[],
        patterns: &["abort", "abort program", "terminate immediately"],
    },
    FunctionSpec {
        name: "atexit",
        header: "stdlib.h",
        return_type: "int",
        params: &[ParamSpec { typ: "void (*)(void)", name: "func", desc: "cleanup function" }],
        patterns: &["atexit", "on exit", "register exit handler"],
    },

    // string.h
    FunctionSpec {
        name: "strlen",
        header: "string.h",
        return_type: "size_t",
        params: &[ParamSpec { typ: "const char *", name: "s", desc: "string" }],
        patterns: &["strlen", "length of", "string length"],
    },
    FunctionSpec {
        name: "strcpy",
        header: "string.h",
        return_type: "char *",
        params: &[
            ParamSpec { typ: "char *", name: "dest", desc: "destination" },
            ParamSpec { typ: "const char *", name: "src", desc: "source" },
        ],
        patterns: &["strcpy", "copy string", "copy string to"],
    },
    FunctionSpec {
        name: "strncpy",
        header: "string.h",
        return_type: "char *",
        params: &[
            ParamSpec { typ: "char *", name: "dest", desc: "destination" },
            ParamSpec { typ: "const char *", name: "src", desc: "source" },
            ParamSpec { typ: "size_t", name: "n", desc: "count" },
        ],
        patterns: &["strncpy", "copy n chars"],
    },
    FunctionSpec {
        name: "strcat",
        header: "string.h",
        return_type: "char *",
        params: &[
            ParamSpec { typ: "char *", name: "dest", desc: "destination" },
            ParamSpec { typ: "const char *", name: "src", desc: "source" },
        ],
        patterns: &["strcat", "concatenate", "append string"],
    },
    FunctionSpec {
        name: "strcmp",
        header: "string.h",
        return_type: "int",
        params: &[
            ParamSpec { typ: "const char *", name: "s1", desc: "left" },
            ParamSpec { typ: "const char *", name: "s2", desc: "right" },
        ],
        patterns: &["strcmp", "compare strings"],
    },
    FunctionSpec {
        name: "memcpy",
        header: "string.h",
        return_type: "void *",
        params: &[
            ParamSpec { typ: "void *", name: "dest", desc: "dest" },
            ParamSpec { typ: "const void *", name: "src", desc: "src" },
            ParamSpec { typ: "size_t", name: "n", desc: "size" },
        ],
        patterns: &["memcpy", "copy bytes"],
    },
    FunctionSpec {
        name: "memset",
        header: "string.h",
        return_type: "void *",
        params: &[
            ParamSpec { typ: "void *", name: "s", desc: "dest" },
            ParamSpec { typ: "int", name: "c", desc: "value" },
            ParamSpec { typ: "size_t", name: "n", desc: "size" },
        ],
        patterns: &["memset", "set bytes"],
    },
    FunctionSpec {
        name: "strstr",
        header: "string.h",
        return_type: "char *",
        params: &[
            ParamSpec { typ: "const char *", name: "haystack", desc: "string" },
            ParamSpec { typ: "const char *", name: "needle", desc: "substring" },
        ],
        patterns: &["strstr", "find substring", "search substring"],
    },
    FunctionSpec {
        name: "strchr",
        header: "string.h",
        return_type: "char *",
        params: &[
            ParamSpec { typ: "const char *", name: "s", desc: "string" },
            ParamSpec { typ: "int", name: "c", desc: "char" },
        ],
        patterns: &["strchr", "find character", "find char"],
    },
    FunctionSpec {
        name: "strrchr",
        header: "string.h",
        return_type: "char *",
        params: &[
            ParamSpec { typ: "const char *", name: "s", desc: "string" },
            ParamSpec { typ: "int", name: "c", desc: "char" },
        ],
        patterns: &["strrchr", "find last character", "find last char"],
    },
    FunctionSpec {
        name: "strtok",
        header: "string.h",
        return_type: "char *",
        params: &[
            ParamSpec { typ: "char *", name: "str", desc: "string" },
            ParamSpec { typ: "const char *", name: "delim", desc: "delimiters" },
        ],
        patterns: &["strtok", "tokenize string", "split string"],
    },
    FunctionSpec {
        name: "strdup",
        header: "string.h",
        return_type: "char *",
        params: &[ParamSpec { typ: "const char *", name: "s", desc: "string" }],
        patterns: &["strdup", "duplicate string", "copy string heap"],
    },
    FunctionSpec {
        name: "strncat",
        header: "string.h",
        return_type: "char *",
        params: &[
            ParamSpec { typ: "char *", name: "dest", desc: "destination" },
            ParamSpec { typ: "const char *", name: "src", desc: "source" },
            ParamSpec { typ: "size_t", name: "n", desc: "count" },
        ],
        patterns: &["strncat", "append n chars"],
    },
    FunctionSpec {
        name: "strncmp",
        header: "string.h",
        return_type: "int",
        params: &[
            ParamSpec { typ: "const char *", name: "s1", desc: "left" },
            ParamSpec { typ: "const char *", name: "s2", desc: "right" },
            ParamSpec { typ: "size_t", name: "n", desc: "count" },
        ],
        patterns: &["strncmp", "compare n chars"],
    },
    FunctionSpec {
        name: "strerror",
        header: "string.h",
        return_type: "char *",
        params: &[ParamSpec { typ: "int", name: "errnum", desc: "error number" }],
        patterns: &["strerror", "error string"],
    },

    // math.h
    FunctionSpec {
        name: "sqrt",
        header: "math.h",
        return_type: "double",
        params: &[ParamSpec { typ: "double", name: "x", desc: "value" }],
        patterns: &["sqrt", "square root"],
    },
    FunctionSpec {
        name: "pow",
        header: "math.h",
        return_type: "double",
        params: &[
            ParamSpec { typ: "double", name: "x", desc: "base" },
            ParamSpec { typ: "double", name: "y", desc: "exp" },
        ],
        patterns: &["pow", "power", "raise to power"],
    },
    FunctionSpec {
        name: "floor",
        header: "math.h",
        return_type: "double",
        params: &[ParamSpec { typ: "double", name: "x", desc: "value" }],
        patterns: &["floor", "round down"],
    },
    FunctionSpec {
        name: "ceil",
        header: "math.h",
        return_type: "double",
        params: &[ParamSpec { typ: "double", name: "x", desc: "value" }],
        patterns: &["ceil", "ceiling", "round up"],
    },
    FunctionSpec {
        name: "round",
        header: "math.h",
        return_type: "double",
        params: &[ParamSpec { typ: "double", name: "x", desc: "value" }],
        patterns: &["round", "rounded"],
    },
    FunctionSpec {
        name: "trunc",
        header: "math.h",
        return_type: "double",
        params: &[ParamSpec { typ: "double", name: "x", desc: "value" }],
        patterns: &["trunc", "truncate"],
    },
    FunctionSpec {
        name: "fabs",
        header: "math.h",
        return_type: "double",
        params: &[ParamSpec { typ: "double", name: "x", desc: "value" }],
        patterns: &["fabs", "absolute value"],
    },
    FunctionSpec {
        name: "fmod",
        header: "math.h",
        return_type: "double",
        params: &[
            ParamSpec { typ: "double", name: "x", desc: "value" },
            ParamSpec { typ: "double", name: "y", desc: "value" },
        ],
        patterns: &["fmod", "floating modulo"],
    },
    FunctionSpec {
        name: "log10",
        header: "math.h",
        return_type: "double",
        params: &[ParamSpec { typ: "double", name: "x", desc: "value" }],
        patterns: &["log10"],
    },
    FunctionSpec {
        name: "log2",
        header: "math.h",
        return_type: "double",
        params: &[ParamSpec { typ: "double", name: "x", desc: "value" }],
        patterns: &["log2"],
    },
    FunctionSpec {
        name: "atan",
        header: "math.h",
        return_type: "double",
        params: &[ParamSpec { typ: "double", name: "x", desc: "value" }],
        patterns: &["atan", "arctan"],
    },
    FunctionSpec {
        name: "atan2",
        header: "math.h",
        return_type: "double",
        params: &[
            ParamSpec { typ: "double", name: "y", desc: "y" },
            ParamSpec { typ: "double", name: "x", desc: "x" },
        ],
        patterns: &["atan2"],
    },
    FunctionSpec {
        name: "asin",
        header: "math.h",
        return_type: "double",
        params: &[ParamSpec { typ: "double", name: "x", desc: "value" }],
        patterns: &["asin", "arcsin"],
    },
    FunctionSpec {
        name: "acos",
        header: "math.h",
        return_type: "double",
        params: &[ParamSpec { typ: "double", name: "x", desc: "value" }],
        patterns: &["acos", "arccos"],
    },
    FunctionSpec {
        name: "abs",
        header: "stdlib.h",
        return_type: "int",
        params: &[ParamSpec { typ: "int", name: "x", desc: "value" }],
        patterns: &["abs", "absolute value"],
    },

    // ctype.h
    FunctionSpec {
        name: "isalpha",
        header: "ctype.h",
        return_type: "int",
        params: &[ParamSpec { typ: "int", name: "c", desc: "char" }],
        patterns: &["isalpha", "is alpha", "is letter"],
    },
    FunctionSpec {
        name: "isdigit",
        header: "ctype.h",
        return_type: "int",
        params: &[ParamSpec { typ: "int", name: "c", desc: "char" }],
        patterns: &["isdigit", "is digit"],
    },
    FunctionSpec {
        name: "isalnum",
        header: "ctype.h",
        return_type: "int",
        params: &[ParamSpec { typ: "int", name: "c", desc: "char" }],
        patterns: &["isalnum", "is alnum", "is alphanumeric"],
    },
    FunctionSpec {
        name: "isspace",
        header: "ctype.h",
        return_type: "int",
        params: &[ParamSpec { typ: "int", name: "c", desc: "char" }],
        patterns: &["isspace", "is space", "is whitespace"],
    },
    FunctionSpec {
        name: "toupper",
        header: "ctype.h",
        return_type: "int",
        params: &[ParamSpec { typ: "int", name: "c", desc: "char" }],
        patterns: &["toupper", "to upper"],
    },
    FunctionSpec {
        name: "tolower",
        header: "ctype.h",
        return_type: "int",
        params: &[ParamSpec { typ: "int", name: "c", desc: "char" }],
        patterns: &["tolower", "to lower"],
    },
    FunctionSpec {
        name: "islower",
        header: "ctype.h",
        return_type: "int",
        params: &[ParamSpec { typ: "int", name: "c", desc: "char" }],
        patterns: &["islower", "is lower"],
    },
    FunctionSpec {
        name: "isupper",
        header: "ctype.h",
        return_type: "int",
        params: &[ParamSpec { typ: "int", name: "c", desc: "char" }],
        patterns: &["isupper", "is upper"],
    },
    FunctionSpec {
        name: "ispunct",
        header: "ctype.h",
        return_type: "int",
        params: &[ParamSpec { typ: "int", name: "c", desc: "char" }],
        patterns: &["ispunct", "is punctuation"],
    },
    FunctionSpec {
        name: "iscntrl",
        header: "ctype.h",
        return_type: "int",
        params: &[ParamSpec { typ: "int", name: "c", desc: "char" }],
        patterns: &["iscntrl", "is control"],
    },
    FunctionSpec {
        name: "isgraph",
        header: "ctype.h",
        return_type: "int",
        params: &[ParamSpec { typ: "int", name: "c", desc: "char" }],
        patterns: &["isgraph", "is graph"],
    },
    FunctionSpec {
        name: "isprint",
        header: "ctype.h",
        return_type: "int",
        params: &[ParamSpec { typ: "int", name: "c", desc: "char" }],
        patterns: &["isprint", "is printable"],
    },

    // time.h
    FunctionSpec {
        name: "time",
        header: "time.h",
        return_type: "time_t",
        params: &[ParamSpec { typ: "time_t *", name: "tloc", desc: "out pointer" }],
        patterns: &["time", "current time"],
    },
    FunctionSpec {
        name: "clock",
        header: "time.h",
        return_type: "clock_t",
        params: &[],
        patterns: &["clock", "cpu clock"],
    },
    FunctionSpec {
        name: "difftime",
        header: "time.h",
        return_type: "double",
        params: &[
            ParamSpec { typ: "time_t", name: "end", desc: "end" },
            ParamSpec { typ: "time_t", name: "start", desc: "start" },
        ],
        patterns: &["difftime", "time difference"],
    },
    FunctionSpec {
        name: "mktime",
        header: "time.h",
        return_type: "time_t",
        params: &[ParamSpec { typ: "struct tm *", name: "timeptr", desc: "tm" }],
        patterns: &["mktime"],
    },
    FunctionSpec {
        name: "localtime",
        header: "time.h",
        return_type: "struct tm *",
        params: &[ParamSpec { typ: "const time_t *", name: "timep", desc: "time" }],
        patterns: &["localtime"],
    },
    FunctionSpec {
        name: "gmtime",
        header: "time.h",
        return_type: "struct tm *",
        params: &[ParamSpec { typ: "const time_t *", name: "timep", desc: "time" }],
        patterns: &["gmtime"],
    },
    FunctionSpec {
        name: "strftime",
        header: "time.h",
        return_type: "size_t",
        params: &[
            ParamSpec { typ: "char *", name: "s", desc: "buffer" },
            ParamSpec { typ: "size_t", name: "max", desc: "max" },
            ParamSpec { typ: "const char *", name: "format", desc: "format" },
            ParamSpec { typ: "const struct tm *", name: "tm", desc: "tm" },
        ],
        patterns: &["strftime", "format time"],
    },
    FunctionSpec {
        name: "ctime",
        header: "time.h",
        return_type: "char *",
        params: &[ParamSpec { typ: "const time_t *", name: "timep", desc: "time" }],
        patterns: &["ctime", "ctime string"],
    },
    FunctionSpec {
        name: "asctime",
        header: "time.h",
        return_type: "char *",
        params: &[ParamSpec { typ: "const struct tm *", name: "tm", desc: "tm" }],
        patterns: &["asctime", "asctime string"],
    },
    FunctionSpec {
        name: "sleep",
        header: "time.h",
        return_type: "unsigned int",
        params: &[ParamSpec { typ: "unsigned int", name: "seconds", desc: "seconds" }],
        patterns: &["sleep", "sleep for", "delay"],
    },

    // signal.h
    FunctionSpec {
        name: "signal",
        header: "signal.h",
        return_type: "void (*)(int)",
        params: &[
            ParamSpec { typ: "int", name: "sig", desc: "signal" },
            ParamSpec { typ: "void (*)(int)", name: "handler", desc: "handler" },
        ],
        patterns: &["signal", "set signal handler"],
    },
    FunctionSpec {
        name: "raise",
        header: "signal.h",
        return_type: "int",
        params: &[ParamSpec { typ: "int", name: "sig", desc: "signal" }],
        patterns: &["raise", "raise signal"],
    },

    // stdarg.h
    FunctionSpec {
        name: "va_start",
        header: "stdarg.h",
        return_type: "void",
        params: &[
            ParamSpec { typ: "va_list", name: "ap", desc: "arg list" },
            ParamSpec { typ: "last", name: "last", desc: "last param" },
        ],
        patterns: &["va_start", "start varargs"],
    },
    FunctionSpec {
        name: "va_arg",
        header: "stdarg.h",
        return_type: "type",
        params: &[
            ParamSpec { typ: "va_list", name: "ap", desc: "arg list" },
            ParamSpec { typ: "type", name: "type", desc: "type" },
        ],
        patterns: &["va_arg", "next vararg"],
    },
    FunctionSpec {
        name: "va_end",
        header: "stdarg.h",
        return_type: "void",
        params: &[ParamSpec { typ: "va_list", name: "ap", desc: "arg list" }],
        patterns: &["va_end", "end varargs"],
    },

    // assert.h
    FunctionSpec {
        name: "assert",
        header: "assert.h",
        return_type: "void",
        params: &[ParamSpec { typ: "int", name: "expression", desc: "expression" }],
        patterns: &["assert"],
    },
];

static CORE_DB: FunctionDatabase = FunctionDatabase { functions: CORE_FUNCS };
