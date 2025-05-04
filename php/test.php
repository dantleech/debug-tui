<?php

$array = [
    'int' => 123,
    'float' => 123.4,
    'string' => "string",
    'bool' => true,
    'more' => [
        'int' => 123,
        'float' => 123.4,
        'string' => "string",
        'bool' => true,
    ],
];
$a = 2;
$foo = $a;
$foo = $array;

$bar = $foo;


(new Foo(true, "foo"))->method3();
call_function("hello");



function call_function(string $hello) {
    $var = 123;
    $obj = new Foo(false, 'good day');
    another_function($hello);
    another_function($hello);
    another_function($hello);
}

function another_function(string $goodbye) {
    echo $goodbye;


    foreach (['one', 'two', 'three', 'four'] as $number) {
        if ($number === 'one') {
            echo "number";
        }
        echo $number;
    }
}

class Foo {
    public function __construct(public bool $true, public string $bar) {}
    public function method1() {
    }
    public function method2() {
    }
    public function method3() {
    }
}

