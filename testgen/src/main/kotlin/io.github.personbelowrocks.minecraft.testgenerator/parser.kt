package io.github.personbelowrocks.minecraft.testgenerator

import kotlin.streams.asSequence
import kotlin.streams.asStream

enum class TokenKind {
    L_PAREN,
    R_PAREN,

    L_BRACKET,
    R_BRACKET,

    EQUALS,
    COMMA,
    JOINT,

    INTEGER,
    DECIMAL,
    STRING,

    LABEL,
}

data class Span(
    val start: UInt,
    val end: UInt,
)

abstract class Token(val span: Span, val kind: TokenKind)
abstract class MulticharToken(s: Span, k: TokenKind): Token(s, k) {

}

class LParen(s: Span, k: TokenKind): Token(s, k)
class RParen(s: Span, k: TokenKind): Token(s, k)

class LBracket(s: Span, k: TokenKind): Token(s, k)
class RBracket(s: Span, k: TokenKind): Token(s, k)

class Equals(s: Span, k: TokenKind): Token(s, k)
class Comma(s: Span, k: TokenKind): Token(s, k)
class Joint(s: Span, k: TokenKind): Token(s, k)

class Integer(val value: Long, s: Span, k: TokenKind): Token(s, k)
class Decimal(val value: Double, s: Span, k: TokenKind): Token(s, k)
class StringToken(val value: String, s: Span, k: TokenKind): Token(s, k)

class Label(val label: String, s: Span, k: TokenKind): Token(s, k)

const val BACKSLASH: Char = '\\'

class LexerCharStream(private val buf: CharArray) {
    private var pos: Int = 0

    fun peek(): Char? = buf.getOrNull(pos)

    fun next(): Char? {
        val ch = buf.getOrNull(pos)
        if (ch != null) pos += 1
        return ch
    }

    fun eof(): Boolean = peek() == null
}

class CommandParser {
    fun lexer(pipelineString: String): List<Token> {
        val stream = LexerCharStream(pipelineString.toCharArray())
        val tokens = mutableListOf<Token>()

        while (!stream.eof()) {

        }

        TODO()
    }
}