// Copyright 2021-2023, Offchain Labs, Inc.
// For license information, see https://github.com/OffchainLabs/nitro/blob/master/LICENSE

package arbmath

import (
	"math"
	"math/big"
	"math/bits"
	"unsafe"

	"github.com/ethereum/go-ethereum/params"
)

// NextPowerOf2 the smallest power of two greater than the input
func NextPowerOf2(value uint64) uint64 {
	return 1 << Log2ceil(value)
}

// NextOrCurrentPowerOf2 the smallest power of no less than the input
func NextOrCurrentPowerOf2(value uint64) uint64 {
	power := NextPowerOf2(value)
	if power == 2*value {
		power /= 2
	}
	return power
}

// Log2ceil the log2 of the int, rounded up
func Log2ceil(value uint64) uint64 {
	return uint64(64 - bits.LeadingZeros64(value))
}

type Signed interface {
	~int | ~int8 | ~int16 | ~int32 | ~int64
}

type Unsigned interface {
	~uint | ~uint8 | ~uint16 | ~uint32 | ~uint64 | ~uintptr
}

type Integer interface {
	Signed | Unsigned
}

type Float interface {
	~float32 | ~float64
}

// Ordered is anything that implements comparison operators such as `<` and `>`.
// Unfortunately, that doesn't include big ints.
type Ordered interface {
	Integer | Float
}

// MinInt the minimum of two ints
func MinInt[T Ordered](value, ceiling T) T {
	if value > ceiling {
		return ceiling
	}
	return value
}

// MaxInt the maximum of one or more ints
func MaxInt[T Ordered](values ...T) T {
	max := values[0]
	for i := 1; i < len(values); i++ {
		value := values[i]
		if value > max {
			max = value
		}
	}
	return max
}

// AbsValue the absolute value of a number
func AbsValue[T Ordered](value T) T {
	if value < 0 {
		return -value // never happens for unsigned types
	}
	return value
}

// Checks if two ints are sufficiently close to one another
func Within[T Unsigned](a, b, bound T) bool {
	min := MinInt(a, b)
	max := MaxInt(a, b)
	return max-min <= bound
}

// Checks if an int belongs to [a, b]
func WithinRange[T Unsigned](value, a, b T) bool {
	return a <= value && value <= b
}

// UintToBig casts an int to a huge
func UintToBig(value uint64) *big.Int {
	return new(big.Int).SetUint64(value)
}

// FloatToBig casts a float to a huge
func FloatToBig(value float64) *big.Int {
	return new(big.Int).SetInt64(int64(value))
}

// UintToBigFloat casts a uint to a big float
func UintToBigFloat(value uint64) *big.Float {
	return new(big.Float).SetPrec(53).SetUint64(value)
}

// BigToUintSaturating casts a huge to a uint, saturating if out of bounds
func BigToUintSaturating(value *big.Int) uint64 {
	if value.Sign() < 0 {
		return 0
	}
	if !value.IsUint64() {
		return math.MaxUint64
	}
	return value.Uint64()
}

// BigToUintOrPanic casts a huge to a uint, panicking if out of bounds
func BigToUintOrPanic(value *big.Int) uint64 {
	if value.Sign() < 0 {
		panic("big.Int value is less than 0")
	}
	if !value.IsUint64() {
		panic("big.Int value exceeds the max Uint64")
	}
	return value.Uint64()
}

// UfracToBigFloat casts an rational to a big float
func UfracToBigFloat(numerator, denominator uint64) *big.Float {
	float := new(big.Float)
	float.Quo(UintToBigFloat(numerator), UintToBigFloat(denominator))
	return float
}

// BigEquals check huge equality
func BigEquals(first, second *big.Int) bool {
	return first.Cmp(second) == 0
}

// BigLessThan check if a huge is less than another
func BigLessThan(first, second *big.Int) bool {
	return first.Cmp(second) < 0
}

// BigGreaterThan check if a huge is greater than another
func BigGreaterThan(first, second *big.Int) bool {
	return first.Cmp(second) > 0
}

// BigGreaterThanOrEqual check if a huge is greater than or equal to another
func BigGreaterThanOrEqual(first, second *big.Int) bool {
	return first.Cmp(second) >= 0
}

// BigMin returns a clone of the minimum of two big integers
func BigMin(first, second *big.Int) *big.Int {
	if BigLessThan(first, second) {
		return new(big.Int).Set(first)
	} else {
		return new(big.Int).Set(second)
	}
}

// BigMax returns a clone of the maximum of two big integers
func BigMax(first, second *big.Int) *big.Int {
	if BigGreaterThan(first, second) {
		return new(big.Int).Set(first)
	} else {
		return new(big.Int).Set(second)
	}
}

// BigAdd add a huge to another
func BigAdd(augend *big.Int, addend *big.Int) *big.Int {
	return new(big.Int).Add(augend, addend)
}

// BigSub subtract from a huge another
func BigSub(minuend *big.Int, subtrahend *big.Int) *big.Int {
	return new(big.Int).Sub(minuend, subtrahend)
}

// BigMul multiply a huge by another
func BigMul(multiplicand *big.Int, multiplier *big.Int) *big.Int {
	return new(big.Int).Mul(multiplicand, multiplier)
}

// BigDiv divide a huge by another
func BigDiv(dividend *big.Int, divisor *big.Int) *big.Int {
	return new(big.Int).Div(dividend, divisor)
}

// BigAbs absolute value of a huge
func BigAbs(value *big.Int) *big.Int {
	return new(big.Int).Abs(value)
}

// BigAddByUint add a uint to a huge
func BigAddByUint(augend *big.Int, addend uint64) *big.Int {
	return new(big.Int).Add(augend, UintToBig(addend))
}

// BigSub subtracts a uint from a huge
func BigSubByUint(minuend *big.Int, subtrahend uint64) *big.Int {
	return new(big.Int).Sub(minuend, UintToBig(subtrahend))
}

// BigMulByFrac multiply a huge by a rational
func BigMulByFrac(value *big.Int, numerator, denominator int64) *big.Int {
	value = new(big.Int).Set(value)
	value.Mul(value, big.NewInt(numerator))
	value.Div(value, big.NewInt(denominator))
	return value
}

// BigMulByUfrac multiply a huge by a rational whose components are non-negative
func BigMulByUfrac(value *big.Int, numerator, denominator uint64) *big.Int {
	value = new(big.Int).Set(value)
	value.Mul(value, new(big.Int).SetUint64(numerator))
	value.Div(value, new(big.Int).SetUint64(denominator))
	return value
}

// BigMulByInt multiply a huge by an integer
func BigMulByInt(multiplicand *big.Int, multiplier int64) *big.Int {
	return new(big.Int).Mul(multiplicand, big.NewInt(multiplier))
}

// BigMulByUint multiply a huge by a unsigned integer
func BigMulByUint(multiplicand *big.Int, multiplier uint64) *big.Int {
	return new(big.Int).Mul(multiplicand, new(big.Int).SetUint64(multiplier))
}

// BigDivByUint divide a huge by an unsigned integer
func BigDivByUint(dividend *big.Int, divisor uint64) *big.Int {
	return BigDiv(dividend, UintToBig(divisor))
}

// BigDivByInt divide a huge by an integer
func BigDivByInt(dividend *big.Int, divisor int64) *big.Int {
	return BigDiv(dividend, big.NewInt(divisor))
}

// BigAddFloat add two big floats together
func BigAddFloat(augend, addend *big.Float) *big.Float {
	return new(big.Float).Add(augend, addend)
}

// BigMulFloat multiply a big float by another
func BigMulFloat(multiplicand, multiplier *big.Float) *big.Float {
	return new(big.Float).Mul(multiplicand, multiplier)
}

// BigFloatMulByUint multiply a big float by an unsigned integer
func BigFloatMulByUint(multiplicand *big.Float, multiplier uint64) *big.Float {
	return new(big.Float).Mul(multiplicand, UintToBigFloat(multiplier))
}

// SaturatingAdd add two integers without overflow
func SaturatingAdd[T Signed](a, b T) T {
	sum := a + b
	if b > 0 && sum < a {
		sum = ^T(0) >> 1
	}
	if b < 0 && sum > a {
		sum = (^T(0) >> 1) + 1
	}
	return sum
}

// SaturatingUAdd add two integers without overflow
func SaturatingUAdd[T Unsigned](a, b T) T {
	sum := a + b
	if sum < a || sum < b {
		sum = ^T(0)
	}
	return sum
}

// SaturatingSub subtract an int64 from another without overflow
func SaturatingSub(minuend, subtrahend int64) int64 {
	return SaturatingAdd(minuend, -subtrahend)
}

// SaturatingUSub subtract an integer from another without underflow
func SaturatingUSub[T Unsigned](a, b T) T {
	if b >= a {
		return 0
	}
	return a - b
}

// SaturatingMul multiply two integers without over/underflow
func SaturatingUMul[T Unsigned](a, b T) T {
	product := a * b
	if b != 0 && product/b != a {
		product = ^T(0)
	}
	return product
}

// SaturatingMul multiply two integers without over/underflow
func SaturatingMul[T Signed](a, b T) T {
	product := a * b
	if b != 0 && product/b != a {
		if (a > 0 && b > 0) || (a < 0 && b < 0) {
			product = ^T(0) >> 1
		} else {
			product = (^T(0) >> 1) + 1
		}
	}
	return product
}

// SaturatingCast cast a uint64 to an int64, clipping to [0, 2^63-1]
func SaturatingCast(value uint64) int64 {
	if value > math.MaxInt64 {
		return math.MaxInt64
	}
	return int64(value)
}

// SaturatingUCast cast a signed integer to an unsigned one, clipping to [0, T::MAX]
func SaturatingUCast[T Unsigned, S Signed](value S) T {
	if value <= 0 {
		return 0
	}
	tSmall := unsafe.Sizeof(T(0)) < unsafe.Sizeof(S(0))
	if tSmall && value >= S(^T(0)) {
		return ^T(0)
	}
	return T(value)
}

func SaturatingCastToUint(value *big.Int) uint64 {
	if value.Sign() < 0 {
		return 0
	}
	if !value.IsUint64() {
		return math.MaxUint64
	}
	return value.Uint64()
}

// ApproxExpBasisPoints return the Maclaurin series approximation of e^x, where x is denominated in basis points.
// This quartic polynomial will underestimate e^x by about 5% as x approaches 20000 bips.
func ApproxExpBasisPoints(value Bips) Bips {
	input := value
	negative := value < 0
	if negative {
		input = -value
	}
	x := uint64(input)

	bips := uint64(OneInBips)
	res := bips + x/4
	res = bips + SaturatingUMul(res, x)/(3*bips)
	res = bips + SaturatingUMul(res, x)/(2*bips)
	res = bips + SaturatingUMul(res, x)/(1*bips)
	if negative {
		return Bips(SaturatingCast(bips * bips / res))
	} else {
		return Bips(SaturatingCast(res))
	}
}

// ApproxSquareRoot return the Newton's method approximation of sqrt(x)
// The error should be no more than 1 for values up to 2^63
func ApproxSquareRoot(value uint64) uint64 {

	if value == 0 {
		return 0
	}

	// ensure our starting approximation's square exceeds the value
	approx := value
	for SaturatingUMul(approx, approx)/2 > value {
		approx /= 2
	}

	for i := 0; i < 4; i++ {
		if approx > value/approx {
			diff := approx - value/approx
			approx = SaturatingUAdd(value/approx, diff/2)
		} else {
			diff := value/approx - approx
			approx = SaturatingUAdd(approx, diff/2)
		}
	}
	return approx
}

// SquareUint returns square of uint
func SquareUint(value uint64) uint64 {
	return value * value
}

// SquareFloat returns square of float
func SquareFloat(value float64) float64 {
	return value * value
}

// BalancePerEther returns balance per ether.
func BalancePerEther(balance *big.Int) float64 {
	balancePerEther, _ := new(big.Float).Quo(new(big.Float).SetInt(balance), new(big.Float).SetFloat64(params.Ether)).Float64()
	return balancePerEther
}
