module miter (
  a,
  b,
  mismatch
);
  input wire a;
  input wire b;
  output wire mismatch;
  wire gate_y;
  wire gold_y;
  wire xor_0_Z;

  AND2 gold_and_inst (
    .A1(a),
    .A2(b),
    .ZN(gold_y)
  );
  OR2 gate_or_inst (
    .A1(a),
    .A2(b),
    .ZN(gate_y)
  );
  XOR2 xor_0 (
    .A(gold_y),
    .B(gate_y),
    .Z(xor_0_Z)
  );

  assign mismatch = xor_0_Z;

endmodule

