use escalier_codegen::d_ts::codegen_d_ts;
use escalier_codegen::js::codegen_js;
use escalier_infer::{infer_prog, Checker, TypeError};
use escalier_old_parser::parse;

fn compile(input: &str) -> (String, String) {
    let program = parse(input).unwrap();
    codegen_js(input, &program)
}

#[test]
fn js_print_multiple_decls() {
    let (js, _) = compile("let foo = \"hello\";\nlet bar = \"world\";");

    insta::assert_snapshot!(js, @r###"
    export const foo = "hello";
    export const bar = "world";
    "###);
}

#[test]
fn unary_minus() {
    let src = r#"
    let negate = (x) => -x;
    "#;

    let (js, _) = compile(src);

    insta::assert_snapshot!(js, @"export const negate = (x)=>-x;
");
}

#[test]
fn fn_with_block_without_return() {
    let src = r#"
    let foo = (x, y) => {
        let z = x + y;
        z
    };
    "#;

    let (js, _) = compile(src);

    insta::assert_snapshot!(js, @r###"
    export const foo = (x, y)=>{
        const z = x + y;
        z;
    };
    "###);
}

#[test]
fn fn_with_block_with_return() {
    let src = r#"
    let foo = (x, y) => {
        let z = x + y;
        return z;
    };
    "#;

    let (js, _) = compile(src);

    insta::assert_snapshot!(js, @r###"
    export const foo = (x, y)=>{
        const z = x + y;
        return z;
    };
    "###);
}

#[test]
fn template_literals() {
    let src = r#"
    let a = `hello, world`;
    let p = {x: 5, y: 10};
    console.log(`p = (${p.x}, ${p.y})`);
    "#;
    let (js, _) = compile(src);

    insta::assert_snapshot!(js, @r###"
    export const a = `hello, world`;
    export const p = {
        x: 5,
        y: 10
    };
    console.log(`p = (${p.x}, ${p.y})`);
    "###);
}

#[test]
fn tagged_template_literals() {
    let src = r#"
    let id = "12345";
    let query = sql`SELECT * FROM users WHERE id = "${id}"`;
    "#;
    let (js, _) = compile(src);

    insta::assert_snapshot!(js, @r###"
    export const id = "12345";
    export const query = sql`SELECT * FROM users WHERE id = "${id}"`;
    "###);
}

#[test]
fn pattern_matching() {
    let src = r#"
    let result = match (count + 1) {
        0 -> "none",
        1 -> "one",
        2 -> "a couple",
        n if (n < 5) -> {
            console.log(`n = ${n}`);
            "a few"
        },
        _ -> {
            console.log("fallthrough");
            "many"
        }
    };
    "#;
    let (js, _) = compile(src);

    insta::assert_snapshot!(js, @r###"
    let $temp_0;
    const $temp_1 = count + 1;
    if ($temp_1 === 0) {
        $temp_0 = "none";
    } else if ($temp_1 === 1) {
        $temp_0 = "one";
    } else if ($temp_1 === 2) {
        $temp_0 = "a couple";
    } else if (n < 5) {
        const n = $temp_1;
        console.log(`n = ${n}`);
        $temp_0 = "a few";
    } else {
        const $temp_2 = $temp_1;
        console.log("fallthrough");
        $temp_0 = "many";
    }
    export const result = $temp_0;
    "###);
}

#[test]
fn pattern_matching_with_disjoint_union() {
    let src = r#"
    type Event = {type: "mousedown", x: number, y: number} | {type: "keydown", key: string};
    declare let event: Event;
    let result = match (event) {
        {type: "mousedown", x, y} -> `mousedown: (${x}, ${y})`,
        {type: "keydown", key} if (key != "Escape") -> key
    };
    "#;
    let (js, _) = compile(src);

    insta::assert_snapshot!(js, @r###"
    ;
    ;
    let $temp_0;
    const $temp_1 = event;
    if ($temp_1.type === "mousedown") {
        const { x, y } = $temp_1;
        $temp_0 = `mousedown: (${x}, ${y})`;
    } else if ($temp_1.type === "keydown" && key !== "Escape") {
        const { key } = $temp_1;
        $temp_0 = key;
    }
    export const result = $temp_0;
    "###);
}

#[test]
// TODO: Have a better error message when there's multiple catch-alls
#[should_panic = "Catchall must appear last in match"]
fn pattern_matching_multiple_catchall_panics() {
    let src = r#"
    let result = match (value) {
        n -> "foo",
        _ -> "bar"
    };
    "#;

    compile(src);
}

#[test]
#[should_panic = "No arms in match"]
fn pattern_matching_no_arms_panics() {
    let src = r#"
    let result = match (value) {
    };
    "#;

    compile(src);
}

#[test]
fn simple_if_else() {
    let src = r#"
    let result = if (cond) {
        console.log("true");
        5
    } else {
        console.log("false");
        10
    };
    "#;
    let (js, _) = compile(src);

    insta::assert_snapshot!(js, @r###"
    let $temp_0;
    if (cond) {
        console.log("true");
        $temp_0 = 5;
    } else {
        console.log("false");
        $temp_0 = 10;
    }
    export const result = $temp_0;
    "###);
}

#[test]
fn simple_if_else_inside_fn() {
    let src = r#"
    let foo = () => {
        let result = if (cond) {
            console.log("true");
            5
        } else {
            console.log("false");
            10
        };
        return result;
    };
    "#;
    let (js, _) = compile(src);

    insta::assert_snapshot!(js, @r###"
    export const foo = ()=>{
        let $temp_0;
        if (cond) {
            console.log("true");
            $temp_0 = 5;
        } else {
            console.log("false");
            $temp_0 = 10;
        }
        const result = $temp_0;
        return result;
    };
    "###);
}

#[test]
fn simple_if_else_inside_fn_as_expr() {
    let src = r#"
    let foo = () => if (cond) {
        console.log("true");
        5
    } else {
        console.log("false");
        10
    };
    "#;
    let (js, _) = compile(src);

    insta::assert_snapshot!(js, @r###"
    let $temp_0;
    if (cond) {
        console.log("true");
        $temp_0 = 5;
    } else {
        console.log("false");
        $temp_0 = 10;
    }
    export const foo = ()=>$temp_0;
    "###);
}

#[test]
fn nested_if_else() {
    let src = r#"
    let result = if (c1) {
        if (c2) {
            5
        } else {
            10
        }
    } else {
        if (c3) {
            "hello"
        } else {
            "world"
        }
    };
    "#;
    let (js, _) = compile(src);

    insta::assert_snapshot!(js, @r###"
    let $temp_0;
    if (c1) {
        let $temp_1;
        if (c2) {
            $temp_1 = 5;
        } else {
            $temp_1 = 10;
        }
        $temp_0 = $temp_1;
    } else {
        let $temp_2;
        if (c3) {
            $temp_2 = "hello";
        } else {
            $temp_2 = "world";
        }
        $temp_0 = $temp_2;
    }
    export const result = $temp_0;
    "###);
}

#[test]
fn multiple_lets_inside_a_function() {
    let src = r#"
    let do_math = () => {
        let x = 5;
        let y = 10;
        let result = x + y;
        return result;
    };
    "#;
    let (js, _) = compile(src);

    insta::assert_snapshot!(js, @r###"
    export const do_math = ()=>{
        const x = 5;
        const y = 10;
        const result = x + y;
        return result;
    };
    "###);
}

#[test]
fn codegen_if_let_with_rename() {
    // TODO: don't allow irrefutable patterns to be used with if-let
    let src = r#"
    let result = if (let {x: a, y: b} = {x: 5, y: 10}) {
        a + b
    };
    "#;
    let (js, _) = compile(src);

    insta::assert_snapshot!(js, @r###"
    let $temp_0;
    const $temp_1 = {
        x: 5,
        y: 10
    };
    {
        const { x: a, y: b } = $temp_1;
        $temp_0 = a + b;
    }export const result = $temp_0;
    "###);
}

#[test]
fn codegen_if_let_refutable_pattern_nested_obj() {
    let src = r#"
    let action = {type: "moveto", point: {x: 5, y: 10}};
    if (let {type: "moveto", point: {x, y}} = action) {
        x + y
    };
    "#;
    let (js, _) = compile(src);

    insta::assert_snapshot!(js, @r###"
    export const action = {
        type: "moveto",
        point: {
            x: 5,
            y: 10
        }
    };
    let $temp_0;
    const $temp_1 = action;
    if ($temp_1.type === "moveto") {
        const { point: { x, y } } = $temp_1;
        $temp_0 = x + y;
    }
    $temp_0;
    "###);
}

#[test]
fn codegen_if_let_with_else() {
    let src = r#"
    declare let a: string | number;
    let result = if (let x is number = a) {
        x + 5
    } else if (let y is string = a) {
        y
    } else {
        true
    };
    "#;
    let (js, _) = compile(src);

    insta::assert_snapshot!(js, @r###"
    ;
    let $temp_0;
    const $temp_1 = a;
    if (typeof $temp_1 === "number") {
        const x = $temp_1;
        $temp_0 = x + 5;
    } else {
        let $temp_2;
        const $temp_3 = a;
        if (typeof $temp_3 === "string") {
            const y = $temp_3;
            $temp_2 = y;
        } else {
            $temp_2 = true;
        }
        $temp_0 = $temp_2;
    }
    export const result = $temp_0;
    "###);
}

#[test]
fn codegen_block_with_multiple_non_let_lines() {
    let src = "let result = do {let x = 5; x + 0; x};";
    let (js, _) = compile(src);

    insta::assert_snapshot!(js, @r###"
    let $temp_0;
    {
        const x = 5;
        x + 0;
        $temp_0 = x;
    }export const result = $temp_0;
    "###);
}

#[test]
fn destructuring_function_object_params() -> Result<(), Vec<TypeError>> {
    let src = r#"
    let foo = ({x, y: b}) => x + b;
    "#;
    let (js, _) = compile(src);

    insta::assert_snapshot!(js, @"export const foo = ({ x, y: b })=>x + b;
");

    let mut program = parse(src).unwrap();
    let mut checker = Checker::default();
    infer_prog(&mut program, &mut checker).unwrap();
    let result = codegen_d_ts(&program, &checker.current_scope)?;

    insta::assert_snapshot!(result, @r###"
    export declare const foo: ({ x, y: b }: {
        readonly x: number;
        readonly y: number;
    }) => number;
    "###);

    Ok(())
}

#[test]
fn destructuring_function_array_params() -> Result<(), Vec<TypeError>> {
    let src = r#"
    let foo = ([a, b]) => a + b;
    "#;
    let (js, _) = compile(src);

    insta::assert_snapshot!(js, @"export const foo = ([a, b])=>a + b;
");

    let mut program = parse(src).unwrap();
    let mut checker = Checker::default();
    infer_prog(&mut program, &mut checker)?;
    let result = codegen_d_ts(&program, &checker.current_scope)?;

    insta::assert_snapshot!(result, @"export declare const foo: ([a, b]: readonly [number, number]) => number;
");

    Ok(())
}

#[test]
fn function_with_rest_param() -> Result<(), Vec<TypeError>> {
    let src = r#"
    let foo = (x: number, ...y: number[]) => x;
    "#;
    let (js, _) = compile(src);

    insta::assert_snapshot!(js, @"export const foo = (x, ...y)=>x;
");

    let mut program = parse(src).unwrap();
    let mut checker = Checker::default();
    infer_prog(&mut program, &mut checker)?;
    let result = codegen_d_ts(&program, &checker.current_scope)?;

    insta::assert_snapshot!(result, @"export declare const foo: (x: number, ...y: readonly number[]) => number;
");

    Ok(())
}

#[test]
fn function_with_optional_param() -> Result<(), Vec<TypeError>> {
    let src = r#"
    let foo = (x: number, y?: number) => x;
    "#;
    let (js, _) = compile(src);

    insta::assert_snapshot!(js, @"export const foo = (x, y)=>x;
");

    let mut program = parse(src).unwrap();
    let mut checker = Checker::default();
    infer_prog(&mut program, &mut checker)?;
    let result = codegen_d_ts(&program, &checker.current_scope)?;

    insta::assert_snapshot!(result, @"export declare const foo: (x: number, y?: number) => number;
");

    Ok(())
}

#[test]
fn function_with_optional_param_and_rest_param() -> Result<(), Vec<TypeError>> {
    let src = r#"
    let foo = (x?: number, ...y: number[]) => x;
    "#;
    let (js, _) = compile(src);

    insta::assert_snapshot!(js, @"export const foo = (x, ...y)=>x;
");

    let mut program = parse(src).unwrap();
    let mut checker = Checker::default();
    infer_prog(&mut program, &mut checker)?;
    let result = codegen_d_ts(&program, &checker.current_scope)?;

    insta::assert_snapshot!(result, @"export declare const foo: (x?: number, ...y: readonly number[]) => number | undefined;
");

    Ok(())
}

#[test]
fn generic_function() -> Result<(), Vec<TypeError>> {
    let src = r#"
    let fst = <T>(a: T, b: T) => a;
    "#;
    let (js, _) = compile(src);

    insta::assert_snapshot!(js, @"export const fst = (a, b)=>a;
");

    let mut program = parse(src).unwrap();
    let mut checker = Checker::default();
    infer_prog(&mut program, &mut checker)?;
    let result = codegen_d_ts(&program, &checker.current_scope)?;

    insta::assert_snapshot!(result, @"export declare const fst: <A>(a: A, b: A) => A;
");

    Ok(())
}

#[test]
fn constrained_generic_function() -> Result<(), Vec<TypeError>> {
    let src = r#"
    let fst = <T extends number | string>(a: T, b: T) => a;
    "#;
    let (js, _) = compile(src);

    insta::assert_snapshot!(js, @"export const fst = (a, b)=>a;
");

    let mut program = parse(src).unwrap();
    let mut checker = Checker::default();
    infer_prog(&mut program, &mut checker).unwrap();
    let result = codegen_d_ts(&program, &checker.current_scope)?;

    insta::assert_snapshot!(result, @"export declare const fst: <A extends number | string>(a: A, b: A) => A;
");

    Ok(())
}

#[test]
fn variable_declaration_with_destructuring() -> Result<(), Vec<TypeError>> {
    let src = r#"
    let [x, y] = [5, 10];
    "#;
    let (js, _) = compile(src);

    insta::assert_snapshot!(js, @r###"
    export const [x, y] = [
        5,
        10
    ];
    "###);

    // TODO: Support destructuring in top-level decls
    let mut program = parse(src).unwrap();
    let mut checker = Checker::default();
    infer_prog(&mut program, &mut checker).unwrap();
    let result = codegen_d_ts(&program, &checker.current_scope)?;

    insta::assert_snapshot!(result, @r###"
    export declare const x: 5;
    export declare const y: 10;
    "###);

    Ok(())
}

#[test]
fn computed_property() {
    let src = r#"
    let p = {x: 5, y: 10};
    let x = p["x"];
    let q = [5, 10];
    let y = q[1];
    "#;
    let (js, _) = compile(src);

    insta::assert_snapshot!(js, @r###"
    export const p = {
        x: 5,
        y: 10
    };
    export const x = p["x"];
    export const q = [
        5,
        10
    ];
    export const y = q[1];
    "###);
}

#[test]
fn partial_application() {
    let src = r#"
    let add = (a, b) => a + b;
    let add5 = (b) => add(5, b);
    let sum = add5(10);
    "#;
    let (js, _) = compile(src);

    insta::assert_snapshot!(js, @r###"
    export const add = (a, b)=>a + b;
    export const add5 = (b)=>add(5, b);
    export const sum = add5(10);
    "###);
}

#[test]
fn partial_application_with_spread() {
    let src = r#"
    let add = (a, b, c) => a + b + c;
    let add5 = (...arg) => add(5, ...args);
    let sum = add5(10, 15);
    "#;
    let (js, _) = compile(src);

    insta::assert_snapshot!(js, @r###"
    export const add = (a, b, c)=>a + b + c;
    export const add5 = (...arg)=>add(5, ...args);
    export const sum = add5(10, 15);
    "###);
}

#[test]
fn spread_args() {
    let src = r#"
    let add = (a, b) => a + b;
    let sum = add(...[5, 10]);
    "#;
    let (js, _) = compile(src);

    insta::assert_snapshot!(js, @r###"
    export const add = (a, b)=>a + b;
    export const sum = add(...[
        5,
        10
    ]);
    "###);
}

#[test]
fn mutable_array() -> Result<(), Vec<TypeError>> {
    let src = r#"
    let arr: mut number[] = [1, 2, 3];
    "#;
    let (js, _) = compile(src);

    insta::assert_snapshot!(js, @r###"
    export const arr = [
        1,
        2,
        3
    ];
    "###);

    let mut program = parse(src).unwrap();
    let mut checker = Checker::default();
    infer_prog(&mut program, &mut checker).unwrap();
    let result = codegen_d_ts(&program, &checker.current_scope)?;

    insta::assert_snapshot!(result, @"export declare const arr: number[];
");

    Ok(())
}

#[test]
fn mutable_obj() -> Result<(), Vec<TypeError>> {
    let src = r#"
    type Point = {mut x: number, mut y: number};
    "#;

    let mut program = parse(src).unwrap();
    let mut checker = Checker::default();
    infer_prog(&mut program, &mut checker).unwrap();
    let result = codegen_d_ts(&program, &checker.current_scope)?;

    // This should be:
    // declare type MutablePoint = {x: number; y: number};
    // declare type Point = Readonly<{x: number; y: number}>;
    insta::assert_snapshot!(result, @r###"
    declare type Point = {
        x: number;
        y: number;
    };
    declare type ReadonlyPoint = {
        readonly x: number;
        readonly y: number;
    };
    "###);

    Ok(())
}

#[test]
fn mutable_indexer() -> Result<(), Vec<TypeError>> {
    let src = r#"
    type Dict = {mut [key: string]: string};
    "#;

    let mut program = parse(src).unwrap();
    let mut checker = Checker::default();
    infer_prog(&mut program, &mut checker).unwrap();
    let result = codegen_d_ts(&program, &checker.current_scope)?;

    insta::assert_snapshot!(result, @r###"
    declare type Dict = {
        [key: string]: string;
    };
    declare type ReadonlyDict = {
        readonly [key: string]: string;
    };
    "###);

    Ok(())
}

#[test]
fn class_with_methods() {
    let src = r#"
    class Foo {
        x: number;
        constructor(self, x) {
            self.x = x;
        }
        foo(self, y) {
            return self.x + y;
        }
        bar(self, y) {
            self.x + y;
        }
    }
    "#;

    let (js, _srcmap) = compile(src);

    insta::assert_snapshot!(js, @r###"
    class Foo {
        constructor(x){
            self.x = x;
        }
        foo(y) {
            return self.x + y;
        }
        bar(y) {
            self.x + y;
        }
    }
    "###);
}

#[test]
fn for_of_loop() -> Result<(), Vec<TypeError>> {
    let src = r#"
    let mut sum: number = 0;
    for (const num in [1, 2, 3]) {
        sum = sum + num;
    }
    "#;

    let (js, _) = compile(src);
    insta::assert_snapshot!(js, @r###"
    export const sum = 0;
    for (const num of [
        1,
        2,
        3
    ]){
        sum = sum + num;
    }
    "###);

    let mut program = parse(src).unwrap();
    let mut checker = Checker::default();
    infer_prog(&mut program, &mut checker).unwrap();
    let result = codegen_d_ts(&program, &checker.current_scope)?;

    insta::assert_snapshot!(result, @r###"export declare const sum: number;
    "###);

    Ok(())
}

#[test]
fn for_loop_inside_fn() -> Result<(), Vec<TypeError>> {
    let src = r#"
    let sum = (arr: number[]) => {
        let mut result: number = 0;
        for (const num in arr) {
            result = result + num;
        }
        return result;
    };
    "#;

    let (js, _) = compile(src);
    insta::assert_snapshot!(js, @r###"
    export const sum = (arr)=>{
        const result = 0;
        for (const num of arr){
            result = result + num;
        }
        return result;
    };
    "###);

    let mut program = parse(src).unwrap();
    let mut checker = Checker::default();
    infer_prog(&mut program, &mut checker).unwrap();
    let result = codegen_d_ts(&program, &checker.current_scope)?;

    insta::assert_snapshot!(result, @"export declare const sum: (arr: readonly number[]) => number;
");

    Ok(())
}

#[test]
fn type_decl_inside_block() -> Result<(), Vec<TypeError>> {
    let src = r#"
    let result = do {
        type Point = {x: number, y: number};
        let p: Point = {x: 5, y: 10};
        p.x + p.y
    };
    "#;

    let (js, _) = compile(src);
    insta::assert_snapshot!(js, @r###"
    let $temp_0;
    {
        const p = {
            x: 5,
            y: 10
        };
        $temp_0 = p.x + p.y;
    }export const result = $temp_0;
    "###);

    let mut program = parse(src).unwrap();
    let mut checker = Checker::default();
    infer_prog(&mut program, &mut checker).unwrap();
    let result = codegen_d_ts(&program, &checker.current_scope)?;

    insta::assert_snapshot!(result, @"export declare const result: number;
");

    Ok(())
}

#[test]
fn type_decl_inside_block_with_escape() -> Result<(), Vec<TypeError>> {
    let src = r#"
    let result = do {
        type Point = {x: number, y: number};
        let p: Point = {x: 5, y: 10};
        p
    };
    "#;

    let (js, _) = compile(src);
    insta::assert_snapshot!(js, @r###"
    let $temp_0;
    {
        const p = {
            x: 5,
            y: 10
        };
        $temp_0 = p;
    }export const result = $temp_0;
    "###);

    let mut program = parse(src).unwrap();
    let mut checker = Checker::default();
    infer_prog(&mut program, &mut checker).unwrap();
    let result = codegen_d_ts(&program, &checker.current_scope)?;

    // TODO: How do we ensure that types defined within a block can't escape?
    insta::assert_snapshot!(result, @"export declare const result: Point;
");

    Ok(())
}

#[test]
fn class_inside_function() -> Result<(), Vec<TypeError>> {
    let src = r#"
    let foo = () => {
        class Point {
            mut x: number;
            mut y: number;
            constructor(self, x, y) {
                self.x = x;
                self.y = y;
            }
        }
        const p = new Point(5, 10);
        return p;
    };
    "#;

    let (js, _) = compile(src);
    insta::assert_snapshot!(js, @r###"
    export const foo = ()=>{
        class Point {
            constructor(x, y){
                self.x = x;
                self.y = y;
            }
        }
        const p = new Point(5, 10);
        return p;
    };
    "###);

    let mut program = parse(src).unwrap();
    let mut checker = Checker::default();
    infer_prog(&mut program, &mut checker)?;
    let result = codegen_d_ts(&program, &checker.current_scope)?;

    insta::assert_snapshot!(result, @"export declare const foo: () => Point;
");

    Ok(())
}

#[test]
#[should_panic = "return statements aren't allowed at the top level"]
fn top_leve_return() {
    let src = r#"
    return 5;
    "#;
    let (js, _) = compile(src);

    insta::assert_snapshot!(js, @r###"
    export const add = (a, b)=>a + b;
    export const sum = add(...[
        5,
        10
    ]);
    "###);
}

#[test]
fn multiple_returns_stress_test() -> Result<(), Vec<TypeError>> {
    let src = r#"
    let foo = (cond: boolean) => {
        let bar = () => {
            if (cond) {
                return 5;
            };
        };
        if (cond) {
            return bar();
        };
        return 10;
    };
    "#;

    let (js, _) = compile(src);
    insta::assert_snapshot!(js, @r###"
    export const foo = (cond)=>{
        const bar = ()=>{
            let $temp_0;
            if (cond) {
                return 5;
            }
            $temp_0;
        };
        let $temp_1;
        if (cond) {
            return bar();
        }
        $temp_1;
        return 10;
    };
    "###);

    let mut program = parse(src).unwrap();
    let mut checker = Checker::default();
    infer_prog(&mut program, &mut checker)?;
    let result = codegen_d_ts(&program, &checker.current_scope)?;

    insta::assert_snapshot!(result, @"export declare const foo: (cond: boolean) => 10 | 5;
");

    Ok(())
}
