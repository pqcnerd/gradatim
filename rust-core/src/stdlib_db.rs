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
