module gate (
  a,
  b,
  y
);

  input a;
  wire a;

  input b;
  wire b;

  output y;
  wire y;

  OR2 or_inst (
    .A1(a),
    .A2(b),
    .ZN(y)
  );

endmodule