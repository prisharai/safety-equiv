module gold (
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

  AND2 and_inst (
    .A1(a),
    .A2(b),
    .ZN(y)
  );

endmodule